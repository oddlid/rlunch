create table country 
(
  country_id uuid primary key default gen_random_uuid(),
  name text not null unique,
  url_id text not null unique,
  currency_suffix text,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
create index on country (url_id);
select trigger_updated_at('country');

create table city 
(
  city_id uuid primary key default gen_random_uuid(),
  country_id uuid not null references country (country_id) on delete cascade,
  name text not null,
  url_id text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
create index on city (url_id);
select trigger_updated_at('city');

create table site 
(
  site_id uuid primary key default gen_random_uuid(),
  city_id uuid not null references city (city_id) on delete cascade,
  name text not null,
  url_id text not null,
  comment text,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
create index on site (url_id);
select trigger_updated_at('site');

create table restaurant 
(
  restaurant_id uuid primary key default gen_random_uuid(),
  site_id uuid not null references site (site_id) on delete cascade,
  restaurant_name text not null,
  comment text,
  address text,
  url text,
  map_url text,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
select trigger_updated_at('restaurant');

create table dish 
(
  dish_id uuid primary key default gen_random_uuid(),
  restaurant_id uuid not null references restaurant (restaurant_id) on delete cascade,
  dish_name text not null,
  description text,
  comment text,
  tags text[],
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
select trigger_updated_at('dish');

-- Insert some default data
-- Values for country/city/site should be static, since it corresponds to what scrapers we have defined.
-- Values for restaurant and dish should be dynamically inserted and deleted by scrapers.

with ins_country as (
  insert into country (name, url_id, currency_suffix)
  values ('Sweden', 'se', 'kr')
  returning *
), ins_city as (
  insert into city (country_id, name, url_id)
  values (
    (select country_id from ins_country),
    'Gothenburg',
    'gbg'
  )
  returning *
), ins_site as (
  insert into site (city_id, name, url_id, comment)
  values (
    (select city_id from ins_city),
    'Lindholmen',
    'lh',
    'GBG Silicon Valley'
  )
  returning *
)
select * from ins_site;
