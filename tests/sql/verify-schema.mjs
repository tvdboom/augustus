import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const root = new URL('../../', import.meta.url);
const requireTool = createRequire(new URL('target/sql-verification/loader.cjs', root));
const { PGlite } = requireTool('@electric-sql/pglite');
const db = new PGlite();
const id = n => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
const host=id(1), guest=id(2), outsider=id(3), recovered=id(4);
const hc=id(11), gc=id(12), oc=id(13), rc=id(14);
const schema=(await readFile(new URL('supabase/schema.sql',root),'utf8'))
  .replace('create extension pg_cron with schema pg_catalog;', 'null;');
const single=await readFile(new URL('target/sql-verification/single.json',root),'utf8');
const multi=await readFile(new URL('target/sql-verification/multi.json',root),'utf8');
try {
  await db.exec(`
    create role anon; create role authenticated; create role service_role bypassrls;
    create schema auth; create table auth.users(id uuid primary key);
    create function auth.uid() returns uuid language sql stable as $$
      select nullif(current_setting('request.jwt.claim.sub',true),'')::uuid
    $$;
    create schema cron;
    create table cron.job(jobid bigint generated always as identity,jobname text unique,schedule text,command text,database text default current_database());
    create table cron.job_run_details(jobid bigint,status text,end_time timestamptz);
    create function cron.schedule(text,text,text) returns bigint language sql as $$
      insert into cron.job(jobname,schedule,command) values($1,$2,$3)
      on conflict(jobname) do update set schedule=excluded.schedule,command=excluded.command returning jobid
    $$;
    create function cron.unschedule(bigint) returns boolean language sql as $$
      with d as(delete from cron.job where jobid=$1 returning jobid) select exists(select 1 from d)
    $$;
  `);
  await db.exec(schema);
  await db.exec("create table public.obsolete(id int); select cron.schedule('external-job','0 0 * * *','select 1'); select cron.schedule('augustus-obsolete','* * * * *','select 1');");
  await db.exec(schema);
  assert.equal((await db.query("select to_regclass('public.obsolete') as r")).rows[0].r,null);
  assert.deepEqual((await db.query('select jobname from cron.job order by jobname')).rows.map(x=>x.jobname),['augustus-delete-expired-games','external-job']);
  assert.equal((await db.query("select count(*)::int n from pg_class where relname like 'augustus_%' and relkind='r' and relrowsecurity")).rows[0].n,3);
  await db.query('insert into auth.users values($1),($2),($3),($4)',[host,guest,outsider,recovered]);
  const call=async(actor,sql,params=[],role='authenticated')=>{
    await db.query("select set_config('request.jwt.claim.sub',$1,false)",[actor??'']);
    await db.exec(`set role ${role}`);
    try {return (await db.query(sql,params)).rows[0]?.result;}finally{await db.exec('reset role');}
  };
  const create=(actor,conn,game=id(101))=>call(actor,'select public.augustus_create($1,$2::smallint,$3::uuid,$4::uuid) result',['Roman',0,conn,game]);
  const open=(actor,game,conn)=>call(actor,'select public.augustus_record($1::uuid,$2::uuid) result',[game,conn]);
  const sync=(actor,g,conn,revision,token)=>call(actor,'select public.augustus_sync($1::uuid,$2::uuid,$3::bigint,$4) result',[g,conn,revision,token]);
  const leave=(actor,g,conn)=>call(actor,'select public.augustus_leave($1::uuid,$2::uuid) result',[g,conn]);
  const save=(actor,g,conn,revision,req,changes)=>call(actor,'select public.augustus_save($1::uuid,$2::uuid,$3::bigint,$4::uuid,$5::jsonb) result',[g,conn,revision,req,JSON.stringify(changes)]);
  const change=(path,before,after)=>({path,before,after});
  await assert.rejects(create(null,oc,id(199)),/Sign in/);
  await assert.rejects(call(null,'select public.augustus_list() result',[],'anon'),/permission denied/);
  await assert.rejects(call(host,'select public.augustus_cleanup() result'),/permission denied/);
  await assert.rejects(call(host,'select * from public.augustus_players'),/permission denied/);
  let lobby=await create(host,hc);
  assert.match(lobby.code,/^[0-9A-HJKMNP-TV-Z]{6}$/);
  assert.match(lobby.recovery_code,/^[0-9A-HJKMNP-TV-Z]{12}$/);
  assert.equal((await create(host,hc)).id,lobby.id,'idempotent creation');
  await assert.rejects(open(outsider,lobby.id,oc),/not a member/);
  await assert.rejects(open(host,lobby.id,id(15)),/another window/);
  await assert.rejects(call(host,'select public.augustus_start($1::uuid,$2::uuid,$3,$4::jsonb) result',[lobby.id,hc,lobby.roster_token,multi]),/Invalid or oversized/);
  const joined=await call(guest,'select public.augustus_join($1,$2,$3::uuid) result',[lobby.code,'Guest',gc]);
  assert.equal(joined.player,1);
  assert.equal(joined.members[1].color,1);
  assert(!joined.members.some(m=>'recovery_code' in m));
  assert.notEqual(joined.recovery_code,lobby.recovery_code);
  await assert.rejects(call(guest,'select public.augustus_start($1::uuid,$2::uuid,$3,$4::jsonb) result',[lobby.id,gc,joined.roster_token,multi]),/Only the host/);
  await assert.rejects(call(host,'select public.augustus_profile($1::uuid,$2::uuid,$3,$4::smallint) result',[lobby.id,hc,'Host',1]),/house color/);
  lobby=await open(host,lobby.id,hc);
  const active=await call(host,'select public.augustus_start($1::uuid,$2::uuid,$3,$4::jsonb) result',[lobby.id,hc,lobby.roster_token,multi]);
  assert.equal(active.status,'active');assert.equal(active.revision,1);assert.equal(active.single_player,false);
  const resume=(actor,request)=>call(actor,'select public.augustus_resume($1::uuid,$2::uuid,$3,$4::uuid) result',[active.id,actor===host?hc:gc,active.roster_token,request]);
  await assert.rejects(resume(guest,id(601)),/Only the host/);
  await assert.rejects(call(host,'select public.augustus_resume($1::uuid,$2::uuid,$3,$4::uuid) result',[active.id,hc,'stale-roster',id(603)]),/roster changed/);
  await leave(guest,active.id,gc);
  const waiting=await open(host,active.id,hc);
  await assert.rejects(call(host,'select public.augustus_resume($1::uuid,$2::uuid,$3,$4::uuid) result',[active.id,hc,waiting.roster_token,id(604)]),/everyone to reconnect/);
  await open(guest,active.id,gc);
  const resumed=await resume(host,id(602));
  assert.deepEqual(resumed,{resume_generation:1});
  assert.deepEqual(await resume(host,id(602)),resumed,'resume retry is idempotent');
  const resumeSync=await sync(guest,active.id,gc,1,active.roster_token);
  assert.equal(resumeSync.resume_generation,1);assert.equal(resumeSync.state,null);assert.deepEqual(resumeSync.events,[]);
  await assert.rejects(call(outsider,'select public.augustus_join($1,$2,$3::uuid) result',[active.code,'Late',oc]),/not accepting/);
  const list=await call(host,'select public.augustus_list() result');assert.equal(list.length,1);assert(!('state' in list[0]));
  assert.equal(list[0].player_count,2);assert.equal(list[0].player_color,0);
  let delta=[change(['clock','month'],0,1)];
  const ack=await save(host,active.id,hc,1,id(201),delta);
  assert.deepEqual(ack,{revision:2});assert.deepEqual(await save(host,active.id,hc,1,id(201),delta),ack);
  await assert.rejects(save(host,active.id,hc,1,id(202),delta),/State conflict/);
  await assert.rejects(call(host,'select public.augustus_save($1::uuid,$2::uuid,2,$3::uuid,null) result',[active.id,hc,id(299)]),/Invalid state delta/);
  const beforeCoin=active.state.campaign.economy.players[1].coin;
  await save(guest,active.id,gc,1,id(203),[change(['campaign','economy','players','1','coin'],beforeCoin,beforeCoin-1)]);
  await assert.rejects(save(guest,active.id,gc,3,id(204),[change(['clock','month'],1,2)]),/Only the host/);
  // A later failure rolls back earlier changes from the same transaction.
  await assert.rejects(save(host,active.id,hc,3,id(205),[change(['clock','month'],1,2),change(['paused'],true,false)]),/State conflict/);
  assert.equal((await open(host,active.id,hc)).state.clock.month,1);
  const catchup=await sync(guest,active.id,gc,1,joined.roster_token);
  assert.equal(catchup.events.length,2);assert.equal(catchup.state,null);assert(!('before' in catchup.events[0].changes[0]));
  const unchanged=await sync(guest,active.id,gc,3,catchup.roster_token);
  assert.deepEqual(unchanged.events,[]);assert.equal(unchanged.members,null);assert.equal(unchanged.state,null);
  const timestamp=(await db.query('select saved_at from public.augustus_games where id=$1',[active.id])).rows[0].saved_at;
  await sync(host,active.id,hc,3,unchanged.roster_token);
  assert.deepEqual((await db.query('select saved_at from public.augustus_games where id=$1',[active.id])).rows[0].saved_at,timestamp);
  await assert.rejects(call(recovered,'select public.augustus_recover($1,$2,$3::uuid) result',[active.code,joined.recovery_code,rc]),/connected/);
  await leave(guest,active.id,gc);
  const restored=await call(recovered,'select public.augustus_recover($1,$2,$3::uuid) result',[active.code.toLowerCase(),joined.recovery_code.replaceAll('-',' ').toLowerCase(),rc]);
  assert.equal(restored.player,1);assert.equal(restored.recovery_code,joined.recovery_code);
  await assert.rejects(open(guest,active.id,gc),/not a member/);
  await leave(host,active.id,hc);assert.equal((await open(host,active.id,hc)).status,'active');
  const originalName=(await open(host,active.id,hc)).state.campaign.economy.provinces[0].name;
  await save(host,active.id,hc,3,id(600),[change(['campaign','economy','provinces','0','name'],originalName,'X'.repeat(300000))]);
  const bigReplay=await sync(host,active.id,hc,3,unchanged.roster_token);
  assert(bigReplay.state);assert.equal(bigReplay.events.length,0);
  assert((await db.query('select coalesce(sum(octet_length(changes::text)),0)::int bytes from public.augustus_events where game_id=$1',[active.id])).rows[0].bytes<=262144);
  console.log('Multiplayer lifecycle, leases, private recovery, authorization, idempotency, atomic merge and compact replay passed.');

  const solo=await create(outsider,oc,id(102));
  assert.equal(solo.status,'lobby');assert.equal(solo.single_player,false);
  const soloActive=await call(outsider,'select public.augustus_start($1::uuid,$2::uuid,$3,$4::jsonb) result',[solo.id,oc,solo.roster_token,single]);
  assert.equal(soloActive.status,'active');assert.equal(soloActive.single_player,true);
  await assert.rejects(call(guest,'select public.augustus_join($1,$2,$3::uuid) result',[solo.code,'Guest',gc]),/not accepting/);
  for(let i=0;i<135;i++) await save(outsider,solo.id,oc,1+i,id(300+i),[change(['clock','month'],i%12,(i+1)%12)]);
  assert.equal((await db.query('select count(*)::int n from public.augustus_events where game_id=$1',[solo.id])).rows[0].n,1);
  const fallback=await sync(outsider,solo.id,oc,1,solo.roster_token);assert(fallback.state);assert.equal(fallback.events.length,0);
  await save(outsider,solo.id,oc,136,id(500),[change(['campaign','senate','winner'],null,0)]);
  assert.equal((await open(outsider,solo.id,oc)).status,'finished');
  await assert.rejects(save(outsider,solo.id,oc,137,id(501),[change(['paused'],false,true)]),/cannot be edited/);
  const disposable=await create(guest,gc,id(103));
  await call(recovered,'select public.augustus_join($1,$2,$3::uuid) result',[disposable.code,'Guest',rc]);
  await leave(guest,disposable.id,gc);
  await assert.rejects(open(recovered,disposable.id,rc),/unavailable/);
  await db.query("update public.augustus_games set finished_at=clock_timestamp()-interval '49 hours' where id=$1",[solo.id]);
  await db.exec("insert into cron.job_run_details select jobid,'succeeded',clock_timestamp()-interval '2 days' from cron.job;");
  await db.exec('select public.augustus_cleanup()');
  assert.equal((await db.query('select count(*)::int n from cron.job_run_details')).rows[0].n,1,'unrelated job logs remain');
  assert.equal((await db.query('select count(*)::int n from public.augustus_events where game_id=$1',[solo.id])).rows[0].n,0);
  await db.query("update public.augustus_games set saved_at=clock_timestamp()-interval '31 days' where id=$1",[active.id]);
  await db.exec('select public.augustus_cleanup()');
  assert.equal((await db.query('select count(*)::int n from public.augustus_games')).rows[0].n,0);
  console.log('Single-player start, bounded retention, catch-up fallback, finished read-only games, lobby deletion and cascading cleanup passed.');
  console.log('Fresh and repeat destructive resets passed; pg_cron execution is stubbed, hosted Supabase was not reset.');
} finally {await db.close();}
