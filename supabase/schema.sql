-- Augustus: ONE complete destructive reset/setup script; no migrations.
-- Paste the entire file in Supabase SQL Editor and run as postgres.
-- WARNING: removes the entire application-facing public schema and saved games.
-- Auth, storage, extensions and unrelated cron jobs remain operational.
-- Enable Anonymous Sign-Ins in Authentication > Sign In / Providers separately.
-- Clients use the publishable key + user JWT. Never ship a secret/service-role key.
-- All backend setup is here. No Edge Functions or separate game server.
-- Rust clients simulate rules; SQL checks membership, leases, lifecycle, delta
-- preconditions and structure. This is not an anti-cheat server for modified clients.
-- Free-tier limits: one snapshot, one replay row for solo games, at most 128 rows
-- and 256 KiB of replay per multiplayer game, 50 resume entries,
-- no Realtime sockets, no snapshot polling/echo, presence renewed every 15 seconds.
-- New Game always opens a joinable lobby; starting alone selects solo saving.
-- Finished games expire after 48 hours; all games expire 30 days after last save.
-- Presence/recovery does not extend retention. Cleanup runs every minute.
-- Verify locally with just sql-check (PostgreSQL/PGlite, scheduler stubbed).
begin;
drop schema if exists public cascade;
create schema public;
grant usage on schema public to postgres, anon, authenticated, service_role;
grant all on schema public to postgres;
revoke create on schema public from public, anon, authenticated;
do $$ begin
  if not exists (select 1 from pg_extension where extname='pg_cron') then
    create extension pg_cron with schema pg_catalog;
  end if;
end $$;
delete from cron.job_run_details where jobid in
  (select jobid from cron.job where database=current_database() and jobname like 'augustus-%');
select cron.unschedule(jobid) from cron.job where database=current_database() and jobname like 'augustus-%';

-- Readable Base32 codes: six public symbols and a twelve-symbol (60-bit)
-- private recovery secret, both without dashes. Omit UUID version/variant bytes so every
-- recovery-code bit comes from gen_random_uuid's cryptographic entropy.
create function public.augustus_random_code(p_symbols integer,p_grouped boolean default false)
returns text language plpgsql volatile set search_path=pg_catalog as $$
declare raw text:=replace(gen_random_uuid()::text,'-',''); entropy bytea;
  alphabet constant text:='0123456789ABCDEFGHJKMNPQRSTVWXYZ'; result text:=''; i integer; bits integer;
begin
  if p_symbols not between 1 and 16 then raise exception 'Invalid code length.'; end if;
  entropy:=decode(substr(raw,1,12)||substr(raw,15,2)||substr(raw,19,6),'hex');
  for i in 0..p_symbols-1 loop
    bits:=(get_byte(entropy,i*5/8)<<8)+case when i*5/8+1<10 then get_byte(entropy,i*5/8+1) else 0 end;
    if p_grouped and i>0 and i%4=0 then result:=result||'-'; end if;
    result:=result||substr(alphabet,((bits>>(11-(i*5)%8))&31)+1,1);
  end loop;
  return result;
end $$;
create function public.augustus_normalize_code(p_code text)
returns text language sql immutable set search_path=pg_catalog as $$
  select translate(upper(regexp_replace(p_code,'[[:space:]-]','','g')),'OIL','011')
$$;

create table public.augustus_games (
  id uuid primary key default gen_random_uuid(),
  code text not null unique check(code ~ '^[0-9A-HJKMNP-TV-Z]{6}$'),
  status text not null default 'lobby' check(status in ('lobby','active','finished')),
  single_player boolean not null default false,
  revision bigint not null default 0 check(revision>=0), state jsonb,
  resume_generation bigint not null default 0 check(resume_generation>=0), resume_request uuid,
  saved_at timestamptz not null default clock_timestamp(), finished_at timestamptz,
  check((status='lobby' and state is null) or (status<>'lobby' and jsonb_typeof(state)='object'))
);
create table public.augustus_players (
  game_id uuid not null references public.augustus_games on delete cascade,
  player smallint not null check(player between 0 and 3),
  user_id uuid not null references auth.users on delete cascade,
  display_name text not null check(char_length(btrim(display_name)) between 1 and 16),
  color smallint not null check(color between 0 and 4),
  recovery_code text not null default public.augustus_random_code(12)
    check(recovery_code ~ '^[0-9A-HJKMNP-TV-Z]{12}$'),
  connection_id uuid, seen_at timestamptz,
  primary key(game_id,player), unique(game_id,user_id), unique(game_id,color), unique(game_id,recovery_code)
);
create table public.augustus_events (
  game_id uuid not null references public.augustus_games on delete cascade,
  revision bigint not null, request_id uuid not null, changes jsonb not null,
  primary key(game_id,revision), unique(game_id,request_id)
);
create index augustus_players_user on public.augustus_players(user_id);
create index augustus_games_saved on public.augustus_games(saved_at);
create index augustus_games_finished on public.augustus_games(finished_at) where finished_at is not null;
alter table public.augustus_games enable row level security;
alter table public.augustus_players enable row level security;
alter table public.augustus_events enable row level security;
-- Direct table access is denied. Only checked SECURITY DEFINER RPCs are exposed.
create function public.augustus_member(p_game_id uuid,p_connection uuid)
returns smallint language plpgsql security definer set search_path=pg_catalog,public as $$
declare m public.augustus_players;
begin
  if auth.uid() is null then raise exception 'Sign in before accessing a game.'; end if;
  select * into m from public.augustus_players where game_id=p_game_id and user_id=auth.uid() for update;
  if not found then raise exception 'This game is unavailable or you are not a member.'; end if;
  if m.connection_id is distinct from p_connection and m.seen_at>clock_timestamp()-interval '45 seconds' then
    raise exception 'Already connected in another window. Close it and wait 45 seconds.';
  end if;
  if p_connection is null then raise exception 'Missing connection identity.'; end if;
  if m.connection_id is distinct from p_connection or m.seen_at is null or m.seen_at<clock_timestamp()-interval '15 seconds' then
    update public.augustus_players set connection_id=p_connection,seen_at=clock_timestamp() where game_id=p_game_id and player=m.player;
  end if;
  return m.player;
end $$;
create function public.augustus_roster(p_game_id uuid)
returns jsonb language sql stable security definer set search_path=pg_catalog,public as $$
  select coalesce(jsonb_agg(jsonb_build_object('player',player,'display_name',display_name,
    'color',color,'connected',coalesce(seen_at>clock_timestamp()-interval '45 seconds',false)) order by player),'[]'::jsonb)
  from public.augustus_players where game_id=p_game_id
$$;
create function public.augustus_record(p_game_id uuid,p_connection uuid)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare n smallint; g public.augustus_games; roster jsonb;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  n:=public.augustus_member(p_game_id,p_connection); roster:=public.augustus_roster(p_game_id);
  return jsonb_build_object('id',g.id,'code',g.code,'status',g.status,'single_player',g.single_player,
    'revision',g.revision,'resume_generation',g.resume_generation,'state',g.state,'members',roster,'player',n,'roster_token',md5(roster::text),
    'recovery_code',(select recovery_code from public.augustus_players where game_id=p_game_id and player=n));
end $$;
create function public.augustus_create(p_name text,p_color smallint,p_connection uuid,p_request_id uuid)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare g uuid:=p_request_id; code_value text;
begin
  if auth.uid() is null then raise exception 'Sign in before creating a game.'; end if;
  if exists(select 1 from public.augustus_games where id=g) then return public.augustus_record(g,p_connection); end if;
  if (select count(*) from public.augustus_players where user_id=auth.uid())>=50 then raise exception 'You already have 50 games.'; end if;
  loop
    code_value:=public.augustus_random_code(6);
    begin insert into public.augustus_games(id,code) values(g,code_value); exit;
    exception when unique_violation then
      if exists(select 1 from public.augustus_games where id=g) then
        return public.augustus_record(g,p_connection);
      end if;
    end;
  end loop;
  insert into public.augustus_players(game_id,player,user_id,display_name,color) values(g,0,auth.uid(),btrim(p_name),p_color);
  return public.augustus_record(g,p_connection);
end $$;
create function public.augustus_join(p_code text,p_name text,p_connection uuid)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare g public.augustus_games; seat smallint; house smallint;
begin
  if auth.uid() is null then raise exception 'Sign in before joining a game.'; end if;
  select * into g from public.augustus_games where code=public.augustus_normalize_code(p_code) for update;
  if not found then raise exception 'Game code not found.'; end if;
  if exists(select 1 from public.augustus_players where game_id=g.id and user_id=auth.uid()) then return public.augustus_record(g.id,p_connection); end if;
  if g.status<>'lobby' or g.single_player then raise exception 'This game is not accepting players.'; end if;
  select min(s)::smallint into seat from generate_series(1,3) s where not exists(select 1 from public.augustus_players where game_id=g.id and player=s);
  if seat is null then raise exception 'The lobby is full (four players).'; end if;
  select min(s)::smallint into house from generate_series(0,4) s where not exists(select 1 from public.augustus_players where game_id=g.id and color=s);
  insert into public.augustus_players(game_id,player,user_id,display_name,color) values(g.id,seat,auth.uid(),btrim(p_name),house);
  return public.augustus_record(g.id,p_connection);
end $$;
create function public.augustus_profile(p_game_id uuid,p_connection uuid,p_name text,p_color smallint)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare n smallint; g public.augustus_games;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  n:=public.augustus_member(p_game_id,p_connection);
  if g.status<>'lobby' then raise exception 'Player cards are locked after starting.'; end if;
  if exists(select 1 from public.augustus_players where game_id=p_game_id and player<>n and color=p_color) then raise exception 'Another player uses that house color.'; end if;
  update public.augustus_players set display_name=btrim(p_name),color=p_color where game_id=p_game_id and player=n;
  return public.augustus_record(p_game_id,p_connection);
end $$;
create function public.augustus_validate_state(s jsonb,n integer)
returns void language plpgsql set search_path=pg_catalog,public as $$
begin
  if s is null or jsonb_typeof(s)<>'object' or octet_length(s::text)>2097152
    or (s->>'version')::integer is distinct from 1 or (s#>>'{campaign,active}')::boolean is distinct from true
    or jsonb_array_length(s#>'{campaign,actors}') is distinct from n
    or jsonb_array_length(s#>'{campaign,economy,players}') is distinct from n
    or jsonb_array_length(s#>'{campaign,defeated}') is distinct from n
    or jsonb_array_length(s#>'{campaign,governance}') is distinct from n
    or jsonb_array_length(s#>'{campaign,economy,provinces}') not between 1 and 1000
    or jsonb_array_length(s#>'{campaign,politics}') is distinct from jsonb_array_length(s#>'{campaign,economy,provinces}')
    or jsonb_array_length(s#>'{campaign,graph}') is distinct from jsonb_array_length(s#>'{campaign,politics}')
    or jsonb_typeof(s->'clock') is distinct from 'object'
    or jsonb_typeof(s#>'{clock,month}') is distinct from 'number'
    or jsonb_typeof(s#>'{clock,speed_step}') is distinct from 'number'
    or jsonb_typeof(s#>'{clock,year}') is distinct from 'number'
    or jsonb_typeof(s#>'{clock,month_progress}') is distinct from 'number'
    or (s#>>'{clock,month}')::integer not between 0 and 11 or (s#>>'{clock,speed_step}')::integer not between -2 and 2
    or (s#>>'{clock,month_progress}')::numeric not between 0 and 3
    or jsonb_typeof(s->'paused') is distinct from 'boolean' then raise exception 'Invalid or oversized Augustus snapshot.'; end if;
end $$;
create function public.augustus_start(p_game_id uuid,p_connection uuid,p_roster_token text,p_state jsonb)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare n smallint; g public.augustus_games; roster jsonb; count_players integer;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  n:=public.augustus_member(p_game_id,p_connection);
  if n<>0 then raise exception 'Only the host can start the campaign.'; end if;
  if g.status<>'lobby' then return public.augustus_record(p_game_id,p_connection); end if;
  roster:=public.augustus_roster(p_game_id);
  if md5(roster::text)<>p_roster_token then raise exception 'The roster changed. Refresh and start again.'; end if;
  count_players:=jsonb_array_length(roster);
  if count_players not between 1 and 4 then raise exception 'A game needs one to four players.'; end if;
  if exists(select 1 from jsonb_array_elements(roster) m where not (m->>'connected')::boolean) then raise exception 'All players must be connected.'; end if;
  for n in 0..count_players-1 loop
    update public.augustus_players set player=n where game_id=p_game_id and player=
      (select player from public.augustus_players where game_id=p_game_id order by player offset n limit 1);
  end loop;
  perform public.augustus_validate_state(p_state,count_players);
  update public.augustus_games set status='active',single_player=(count_players=1),state=p_state,revision=1,saved_at=clock_timestamp() where id=p_game_id;
  return public.augustus_record(p_game_id,p_connection);
end $$;
create function public.augustus_save(p_game_id uuid,p_connection uuid,p_revision bigint,p_request_id uuid,p_changes jsonb)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare n smallint; g public.augustus_games; next_state jsonb; delta jsonb; path text[]; replay jsonb:='[]'; prior bigint; count_players integer;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  n:=public.augustus_member(p_game_id,p_connection);
  select revision into prior from public.augustus_events where game_id=p_game_id and request_id=p_request_id;
  if found then return jsonb_build_object('revision',prior); end if;
  if g.status<>'active' then raise exception 'This campaign cannot be edited.'; end if;
  if p_revision<greatest(1,g.revision-127) or p_revision>g.revision then raise exception 'State conflict. Sync and retry.'; end if;
  if jsonb_typeof(p_changes) is distinct from 'array' or jsonb_array_length(p_changes) not between 1 and 20000 or octet_length(p_changes::text)>2097152 then raise exception 'Invalid state delta.'; end if;
  next_state:=g.state;
  for delta in select value from jsonb_array_elements(p_changes) loop
    if jsonb_typeof(delta->'path') is distinct from 'array' or not(delta?'before' and delta?'after') then raise exception 'Invalid delta.'; end if;
    select array_agg(value) into path from jsonb_array_elements_text(delta->'path');
    if coalesce(cardinality(path),0) not between 1 and 16 or path[1] not in ('campaign','clock','paused') then raise exception 'Invalid delta path.'; end if;
    if n<>0 and path[1] in ('clock','paused') then raise exception 'Only the host controls time.'; end if;
    if next_state#>path is distinct from delta->'before' then raise exception 'State conflict. Sync and retry.'; end if;
    next_state:=jsonb_set(next_state,path,delta->'after',false);
    replay:=replay||jsonb_build_array(jsonb_build_object('path',delta->'path','after',delta->'after'));
  end loop;
  count_players:=(select count(*) from public.augustus_players where game_id=p_game_id);
  perform public.augustus_validate_state(next_state,count_players);
  if n<>0 and next_state#>'{campaign,economy,month}' is distinct from g.state#>'{campaign,economy,month}' then raise exception 'Only the host advances the campaign.'; end if;
  update public.augustus_games set state=next_state,revision=revision+1,saved_at=clock_timestamp(),
    status=case when next_state#>'{campaign,senate,winner}'<>'null'::jsonb then 'finished' else 'active' end,
    finished_at=case when next_state#>'{campaign,senate,winner}'<>'null'::jsonb then clock_timestamp() else null end
    where id=p_game_id returning revision into prior;
  -- Solo games need only an idempotency receipt. Oversized multiplayer updates
  -- also retain a receipt; their readers catch up through the current snapshot.
  insert into public.augustus_events values(p_game_id,prior,p_request_id,
    case when g.single_player or octet_length(replay::text)>262144 then 'null'::jsonb else replay end);
  delete from public.augustus_events where game_id=p_game_id
    and revision <= prior-(case when g.single_player then 1 else 128 end);
  -- Bound bytes as well as rows; always retain the latest idempotency receipt.
  delete from public.augustus_events where game_id=p_game_id and revision < coalesce(
    (select min(revision) from (
      select revision,sum(octet_length(changes::text)) over(order by revision desc) as bytes
        from public.augustus_events where game_id=p_game_id
    ) recent where bytes<=262144),prior);
  return jsonb_build_object('revision',prior); -- No successful-save snapshot echo.
end $$;
create function public.augustus_resume(p_game_id uuid,p_connection uuid,p_roster_token text,p_request_id uuid)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare g public.augustus_games; n smallint; roster jsonb;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  n:=public.augustus_member(p_game_id,p_connection);
  if n<>0 then raise exception 'Only the host can resume the game.'; end if;
  if g.status<>'active' or p_request_id is null then raise exception 'This game cannot be resumed.'; end if;
  if g.resume_request=p_request_id then return jsonb_build_object('resume_generation',g.resume_generation); end if;
  roster:=public.augustus_roster(p_game_id);
  if md5(roster::text) is distinct from p_roster_token then raise exception 'The player roster changed. Please retry.'; end if;
  if exists(select 1 from jsonb_array_elements(roster) m where not (m->>'connected')::boolean) then
    raise exception 'Wait for everyone to reconnect before resuming.';
  end if;
  update public.augustus_games set resume_generation=resume_generation+1,resume_request=p_request_id
    where id=p_game_id returning * into g;
  return jsonb_build_object('resume_generation',g.resume_generation);
end $$;
create function public.augustus_sync(p_game_id uuid,p_connection uuid,p_revision bigint,p_roster_token text)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare g public.augustus_games; roster jsonb; token text; events jsonb; fallback boolean;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  perform public.augustus_member(p_game_id,p_connection);
  roster:=public.augustus_roster(p_game_id); token:=md5(roster::text);
  if p_revision<0 or p_revision>g.revision then raise exception 'Invalid replay cursor.'; end if;
  fallback:=p_revision<g.revision and (
    not exists(select 1 from public.augustus_events where game_id=p_game_id and revision=p_revision+1)
    or exists(select 1 from public.augustus_events where game_id=p_game_id and revision>p_revision and changes='null'::jsonb));
  select coalesce(jsonb_agg(jsonb_build_object('revision',revision,'changes',changes) order by revision),'[]'::jsonb) into events
    from public.augustus_events where game_id=p_game_id and revision>p_revision and not fallback;
  if not fallback and p_revision<g.revision and octet_length(events::text)>octet_length(g.state::text) then
    fallback:=true;events:='[]'; -- Use the smaller representation for long catch-up.
  end if;
  return jsonb_build_object('revision',g.revision,'status',g.status,'resume_generation',g.resume_generation,'roster_token',token,
    'members',case when token is distinct from p_roster_token then roster else null end,'events',events,'state',case when fallback then g.state else null end);
end $$;
create function public.augustus_list()
returns jsonb language sql stable security definer set search_path=pg_catalog,public as $$
  select coalesce(jsonb_agg(row_to_json(q)),'[]'::jsonb) from
    (select g.id,g.code,g.status,g.saved_at,p.display_name,p.color as player_color,
      coalesce((g.state#>>'{campaign,economy,month}')::integer,0) as turn,
      (select count(*) from public.augustus_players where game_id=g.id) as player_count
      from public.augustus_games g join public.augustus_players p on p.game_id=g.id
      where p.user_id=auth.uid() and g.status<>'lobby' order by g.saved_at desc limit 50) q
$$;
create function public.augustus_recover(p_code text,p_recovery_code text,p_connection uuid)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare g public.augustus_games; m public.augustus_players;
begin
  if auth.uid() is null then raise exception 'Sign in before recovering a game.'; end if;
  select * into g from public.augustus_games where code=public.augustus_normalize_code(p_code) for update;
  select * into m from public.augustus_players where game_id=g.id and replace(recovery_code,'-','')=public.augustus_normalize_code(p_recovery_code) for update;
  if g.status='lobby' or m.game_id is null then raise exception 'Game or recovery code not found.'; end if;
  if m.seen_at>clock_timestamp()-interval '45 seconds' and m.connection_id is distinct from p_connection then raise exception 'That player is connected. Close the other window and wait 45 seconds.'; end if;
  if exists(select 1 from public.augustus_players where game_id=g.id and user_id=auth.uid() and player<>m.player) then raise exception 'This identity already controls another player in this game.'; end if;
  update public.augustus_players set user_id=auth.uid(),connection_id=p_connection,seen_at=clock_timestamp() where game_id=g.id and player=m.player;
  return public.augustus_record(g.id,p_connection);
end $$;
create function public.augustus_leave(p_game_id uuid,p_connection uuid)
returns jsonb language plpgsql security definer set search_path=pg_catalog,public as $$
declare n smallint; g public.augustus_games;
begin
  select * into g from public.augustus_games where id=p_game_id for update;
  if not found then return jsonb_build_object('ok',true); end if;
  n:=public.augustus_member(p_game_id,p_connection);
  if g.status='lobby' then
    if n=0 then delete from public.augustus_games where id=p_game_id;
    else delete from public.augustus_players where game_id=p_game_id and player=n; end if;
  else update public.augustus_players set seen_at=null,connection_id=null where game_id=p_game_id and player=n; end if;
  return jsonb_build_object('ok',true);
end $$;
create function public.augustus_cleanup()
returns void language sql security definer set search_path=pg_catalog,public as $$
  delete from public.augustus_games where saved_at<clock_timestamp()-interval '30 days' or finished_at<clock_timestamp()-interval '48 hours'
  ; delete from cron.job_run_details where jobid in
    (select jobid from cron.job where database=current_database() and jobname='augustus-delete-expired-games')
    and end_time<clock_timestamp()-interval '1 day'
$$;
revoke all on all tables in schema public from public,anon,authenticated;
revoke all on all functions in schema public from public,anon,authenticated;
grant execute on function public.augustus_create(text,smallint,uuid,uuid),public.augustus_join(text,text,uuid),public.augustus_record(uuid,uuid),
  public.augustus_profile(uuid,uuid,text,smallint),public.augustus_start(uuid,uuid,text,jsonb),public.augustus_save(uuid,uuid,bigint,uuid,jsonb),
  public.augustus_resume(uuid,uuid,text,uuid),
  public.augustus_sync(uuid,uuid,bigint,text),public.augustus_list(),public.augustus_recover(text,text,uuid),public.augustus_leave(uuid,uuid) to authenticated;
select cron.schedule('augustus-delete-expired-games','* * * * *','select public.augustus_cleanup()');
notify pgrst,'reload schema';
commit;
