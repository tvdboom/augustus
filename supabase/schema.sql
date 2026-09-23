-- Initial Augustus lobby schema. The client transport will be connected once
-- the Supabase project URL and publishable key are supplied.

create table if not exists public.games (
    id uuid primary key default gen_random_uuid(),
    code text not null unique check (code ~ '^[A-Z0-9-]{4,16}$'),
    host_id uuid not null references auth.users(id) on delete cascade,
    status text not null default 'lobby' check (status in ('lobby', 'started', 'finished')),
    created_at timestamptz not null default now()
);

create table if not exists public.game_players (
    game_id uuid not null references public.games(id) on delete cascade,
    user_id uuid not null references auth.users(id) on delete cascade,
    display_name text not null check (length(display_name) between 1 and 32),
    color text not null default 'red',
    joined_at timestamptz not null default now(),
    primary key (game_id, user_id)
);

create index if not exists game_players_user_id_idx on public.game_players(user_id);

alter table public.games enable row level security;
alter table public.game_players enable row level security;

drop policy if exists "players can read their games" on public.games;
create policy "players can read their games" on public.games
    for select to authenticated
    using (host_id = (select auth.uid()));

drop policy if exists "hosts can create games" on public.games;
create policy "hosts can create games" on public.games
    for insert to authenticated
    with check (host_id = (select auth.uid()));

drop policy if exists "players can read their lobby roster" on public.game_players;
create policy "players can read their lobby roster" on public.game_players
    for select to authenticated
    using (
        user_id = (select auth.uid())
        or exists (
            select 1 from public.games g
            where g.id = game_players.game_id and g.host_id = (select auth.uid())
        )
    );

drop policy if exists "players can update their own lobby card" on public.game_players;
create policy "players can update their own lobby card" on public.game_players
    for update to authenticated
    using (user_id = (select auth.uid()))
    with check (user_id = (select auth.uid()));

grant select, insert on public.games to authenticated;
grant select, update on public.game_players to authenticated;

-- Creating or joining a lobby will be exposed through validated RPCs when the
-- project credentials are available; do not open direct anonymous inserts.
