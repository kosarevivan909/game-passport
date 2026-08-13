-- Game Passport profile storage. Run this in Supabase SQL Editor once.
create extension if not exists pgcrypto;

create table if not exists public.profiles (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  name text not null check (char_length(trim(name)) between 1 and 40),
  game text not null check (game in ('cs2', 'dota2', 'pubg')),
  settings jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists profiles_user_id_updated_at_idx
  on public.profiles (user_id, updated_at desc);

alter table public.profiles enable row level security;

create policy "Users can read own profiles" on public.profiles
  for select using (auth.uid() = user_id);
create policy "Users can insert own profiles" on public.profiles
  for insert with check (auth.uid() = user_id);
create policy "Users can update own profiles" on public.profiles
  for update using (auth.uid() = user_id) with check (auth.uid() = user_id);
create policy "Users can delete own profiles" on public.profiles
  for delete using (auth.uid() = user_id);

create or replace function public.set_updated_at()
returns trigger language plpgsql security invoker set search_path = '' as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create or replace function public.enforce_profile_limit()
returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if (select count(*) from public.profiles where user_id = new.user_id) >= 5 then
    raise exception 'A user can have at most 5 profiles' using errcode = 'check_violation';
  end if;
  return new;
end;
$$;

drop trigger if exists profiles_set_updated_at on public.profiles;
create trigger profiles_set_updated_at before update on public.profiles
for each row execute function public.set_updated_at();

drop trigger if exists profiles_limit on public.profiles;
create trigger profiles_limit before insert on public.profiles
for each row execute function public.enforce_profile_limit();
