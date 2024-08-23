create table country 
(
  country_id uuid primary key default uuid_generate_v1mc(),
  country_name text not null,
  currency_suffix text,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
select trigger_updated_at('country');

create table city 
(
  city_id uuid primary key default uuid_generate_v1mc(),
  country uuid not null references country (country_id) on delete cascade,
  city_name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
select trigger_updated_at('city');

create table site 
(
  site_id uuid primary key default uuid_generate_v1mc(),
  city uuid not null references city (city_id) on delete cascade,
  site_name text not null,
  comment text,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
select trigger_updated_at('site');

create table restaurant 
(
  restaurant_id uuid primary key default uuid_generate_v1mc(),
  site uuid not null references site (site_id) on delete cascade,
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
  dish_id uuid primary key default uuid_generate_v1mc(),
  restaurant uuid not null references restaurant (restaurant_id) on delete cascade,
  dish_name text not null,
  description text,
  comment text,
  tags text[],
  created_at timestamptz not null default now(),
  updated_at timestamptz
);
select trigger_updated_at('dish');

-- insert some default data
insert into country(country_name, currency_suffix) values('Sweden', 'kr');

insert into city(country, city_name) 
select country_id, 'Gothenburg'
from country 
where country_name = 'Sweden';
