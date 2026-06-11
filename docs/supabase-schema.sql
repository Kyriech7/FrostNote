-- FrostNote cloud sync schema for Supabase.
-- Run this in the Supabase SQL editor after creating the project.
-- Custom UID: each user picks a unique text identifier at registration.
-- The profiles table maps auth.users UUID → user-chosen custom_uid.

-- ============================================================================
-- Profiles table: maps Supabase Auth UUID to user-chosen custom UID
-- ============================================================================

create table if not exists public.profiles (
  user_id uuid primary key references auth.users(id) on delete cascade,
  custom_uid text not null unique
);

-- Auto-create a profile row when a user signs up.
-- The custom_uid must be passed via signUp options.data.custom_uid.
create or replace function public.handle_new_user()
returns trigger as $$
begin
  insert into public.profiles (user_id, custom_uid)
  values (new.id, new.raw_user_meta_data->>'custom_uid');
  return new;
end;
$$ language plpgsql security definer;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();

-- ============================================================================
-- RLS helper: resolve custom_uid for the current authenticated user
-- ============================================================================

create or replace function public.get_my_custom_uid()
returns text
language sql
stable
security definer
as $$
  select custom_uid from public.profiles where user_id = (select auth.uid());
$$;

-- ============================================================================
-- RPC: check whether a custom_uid is available (called before signup)
-- ============================================================================

create or replace function public.check_custom_uid_available(uid text)
returns boolean
language sql
security definer
as $$
  select not exists (select 1 from public.profiles where custom_uid = uid);
$$;

-- ============================================================================
-- Records table: using user-chosen custom_uid as user_id (text)
-- ============================================================================

drop table if exists public.records;

create table public.records (
  id text primary key,
  user_id text not null references public.profiles(custom_uid) on delete cascade,
  type text not null check (type in ('note', 'todo')),
  content text not null,
  date text not null,
  status text check (status in ('pending', 'done') or status is null),
  created_at text not null,
  updated_at text not null,
  completed_at text,
  rolled_over_from_date text,
  deleted_at text
);

-- ============================================================================
-- Indexes
-- ============================================================================

create index records_user_id_idx on public.records (user_id);
create index records_user_updated_at_idx on public.records (user_id, updated_at desc);

-- ============================================================================
-- Row Level Security
-- ============================================================================

alter table public.records enable row level security;

-- Select: only own records
drop policy if exists "records_select_own" on public.records;
create policy "records_select_own"
on public.records
for select
to authenticated
using (public.get_my_custom_uid() = user_id);

-- Insert: only with own custom_uid
drop policy if exists "records_insert_own" on public.records;
create policy "records_insert_own"
on public.records
for insert
to authenticated
with check (public.get_my_custom_uid() = user_id);

-- Update: only own records
drop policy if exists "records_update_own" on public.records;
create policy "records_update_own"
on public.records
for update
to authenticated
using (public.get_my_custom_uid() = user_id)
with check (public.get_my_custom_uid() = user_id);

-- Delete: only own records
drop policy if exists "records_delete_own" on public.records;
create policy "records_delete_own"
on public.records
for delete
to authenticated
using (public.get_my_custom_uid() = user_id);
