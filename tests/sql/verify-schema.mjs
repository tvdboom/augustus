import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const root = new URL("../../", import.meta.url);
// Match CI's disposable tooling without adding dependencies to the game.
const requireTool = createRequire(new URL("target/sql-verification/loader.cjs", root));
const { PGlite } = requireTool("@electric-sql/pglite");
const db = new PGlite();

const host = "00000000-0000-0000-0000-000000000001";
const guest = "00000000-0000-0000-0000-000000000002";
const outsider = "00000000-0000-0000-0000-000000000003";
const game = "10000000-0000-0000-0000-000000000001";
const otherGame = "10000000-0000-0000-0000-000000000002";
const newGame = "10000000-0000-0000-0000-000000000003";

try {
  // Supply Supabase's managed auth boundary; run the actual application schema.
  await db.exec(`
    create role anon;
    create role authenticated;
    create schema auth;
    create table auth.users (id uuid primary key);
    create function auth.uid() returns uuid language sql stable as $$
      select nullif(current_setting('request.jwt.claim.sub', true), '')::uuid
    $$;
    grant usage on schema auth to authenticated;
  `);
  const schema = await readFile(new URL("supabase/schema.sql", root), "utf8");
  await db.exec(schema);
  await db.exec(schema);
  const secured = await db.query(`
    select relname from pg_class
    where oid in ('public.games'::regclass, 'public.game_players'::regclass)
      and relrowsecurity order by relname
  `);
  assert.deepEqual(secured.rows.map(row => row.relname), ["game_players", "games"]);
  assert.equal((await db.query(
    "select count(*)::int as n from pg_policies where schemaname = 'public'",
  )).rows[0].n, 4);
  console.log("Fresh installation, repeat installation, and lobby RLS passed.");

  await db.query("insert into auth.users values ($1), ($2), ($3)", [host, guest, outsider]);
  const asUser = async (actor, sql, params = [], role = "authenticated") => {
    await db.query("select set_config('request.jwt.claim.sub', $1, false)", [actor ?? ""]);
    await db.exec(`set role ${role}`);
    try {
      return await db.query(sql, params);
    } finally {
      await db.exec("reset role");
    }
  };
  const create = (actor, id, code, owner = actor) => asUser(actor,
    "insert into public.games (id, code, host_id) values ($1, $2, $3) returning status",
    [id, code, owner]);
  assert.equal((await create(host, game, "ABCDEF")).rows[0].status, "lobby");
  await create(outsider, otherGame, "UVWXYZ");
  await assert.rejects(create(guest, newGame, "GUEST1", host), /row-level security/);
  await assert.rejects(create(host, newGame, "bad"), /check constraint/);
  await assert.rejects(create(host, newGame, "ABCDEF"), /games_code_key/);
  await assert.rejects(asUser(null, "select * from public.games", [], "anon"), /permission denied/);
  await assert.rejects(asUser(null,
    "insert into public.games (code, host_id) values ($1, $2)",
    ["ANON01", host], "anon"), /permission denied/);
  assert.deepEqual((await asUser(host, "select id from public.games")).rows, [{ id: game }]);
  assert.deepEqual((await asUser(guest, "select id from public.games")).rows, []);
  assert.deepEqual((await asUser(outsider, "select id from public.games")).rows, [{ id: otherGame }]);
  await assert.rejects(asUser(host, "update public.games set status = 'started'"), /permission denied/);
  await assert.rejects(asUser(host, "delete from public.games"), /permission denied/);
  console.log("Host-only game access, creation constraints, and anonymous/write restrictions passed.");

  // Joining is reserved for future RPCs, so seed rosters as the database owner.
  await db.query(`
    insert into public.game_players (game_id, user_id, display_name)
    values ($1, $2, 'Host'), ($1, $3, 'Guest'), ($4, $5, 'Outsider')
  `, [game, host, guest, otherGame, outsider]);
  assert.deepEqual((await asUser(host,
    "select display_name from public.game_players order by display_name",
  )).rows.map(row => row.display_name), ["Guest", "Host"]);
  assert.deepEqual((await asUser(guest,
    "select display_name from public.game_players",
  )).rows, [{ display_name: "Guest" }]);
  assert.deepEqual((await asUser(outsider,
    "select display_name from public.game_players",
  )).rows, [{ display_name: "Outsider" }]);
  assert.equal((await asUser(guest,
    "update public.game_players set display_name = 'Renamed', color = 'blue' where user_id = $1 returning color",
    [guest],
  )).rows[0].color, "blue");
  assert.deepEqual((await asUser(host,
    "update public.game_players set display_name = 'Changed' where user_id = $1 returning user_id",
    [guest],
  )).rows, []);
  await assert.rejects(asUser(guest,
    "update public.game_players set user_id = $1 where user_id = $2", [outsider, guest],
  ), /row-level security/);
  await assert.rejects(asUser(guest,
    "update public.game_players set display_name = '' where user_id = $1", [guest],
  ), /check constraint/);
  await assert.rejects(asUser(guest,
    "insert into public.game_players (game_id, user_id, display_name) values ($1, $2, 'New')",
    [otherGame, guest],
  ), /permission denied/);
  await assert.rejects(asUser(null, "select * from public.game_players", [], "anon"), /permission denied/);
  console.log("Roster visibility, own-card updates, identity checks, and direct-join restrictions passed.");

  await db.query("delete from auth.users where id = $1", [host]);
  assert.deepEqual((await db.query("select id from public.games")).rows, [{ id: otherGame }]);
  assert.deepEqual((await db.query("select user_id from public.game_players")).rows, [{ user_id: outsider }]);
  assert.equal((await db.query("select count(*)::int as n from auth.users")).rows[0].n, 2);
  console.log("Host deletion cascades through its game and roster while preserving unrelated users and games.");
} finally {
  await db.close();
}
