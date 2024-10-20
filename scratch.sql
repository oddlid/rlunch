-- Get a full listing of all restaurants and dishes in a site, including parent
-- city and country.
-- Good in they way that you can get everything in one single query.
-- Bad in the way that all parent info is duplicated for each dish, which could be affecting 
-- performance due to increased bandwith and memory usage.
with si as (
    select * from site where site_id = 'e0a4f989-bc6e-477a-b34e-c2df773a1c55' -- replace with valid ID
), ci as (
    select * from city where city_id = (select city_id from si)
), co as (
    select * from country where country_id = (select country_id from ci)
), r as (
    select * from restaurant where site_id = (select site_id from si)
), d as (
    select * from dish where restaurant_id in (select restaurant_id from r)
)
select 
    co.country_id country_id,
    ci.city_id city_id,
    si.site_id site_id,
    r.restaurant_id restaurant_id,
    d.dish_id dish_id,
    co.name country_name,
    co.url_id country_url_id,
    co.currency_suffix currency_suffix,
    co.created_at country_created_at,
    ci.name city_name,
    ci.url_id city_url_id,
    ci.created_at city_created_at,
    si.name site_name,
    si.url_id site_url_id,
    si.comment site_comment,
    si.created_at site_created_at,
    r.restaurant_name restaurant_name,
    r.comment restaurant_comment,
    r.address restaurant_address,
    r.url restaurant_url,
    r.map_url restaurant_map_url,
    r.created_at restaurant_created_at,
    d.dish_name dish_name,
    d.description dish_description,
    d.comment dish_comment,
    d.tags dish_tags,
    d.price dish_price,
    d.created_at dish_created_at
from si, ci, co, r, d;

