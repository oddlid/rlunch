-- create extension if not exists "uuid-ossp";

create or replace function set_updated_at()
    returns trigger as
$$
begin
    NEW.updated_at = now();
    return NEW;
end;
$$ language plpgsql;

create or replace function trigger_updated_at(tablename regclass)
    returns void as
$$
begin
    execute format('CREATE TRIGGER set_updated_at
        BEFORE UPDATE
        ON %s
        FOR EACH ROW
        WHEN (OLD is distinct from NEW)
        EXECUTE FUNCTION set_updated_at();', tablename);
end;
$$ language plpgsql;

-- Finally, this is a text collation that sorts text case-insensitively, useful for `UNIQUE` indexes
-- over things like usernames and emails, without needing to remember to do case-conversion.
-- create collation case_insensitive (provider = icu, locale = 'und-u-ks-level2', deterministic = false);

-- Function to unnest but keep the inner arrays of tags
-- Taken from: https://dbfiddle.uk/3cD1er77
-- unnest Nd array to 1d arrays (LANGUAGE plpgsql)
CREATE OR REPLACE FUNCTION unnest_nd_1d(a anyarray, OUT a_1d anyarray)
  RETURNS SETOF anyarray
  LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE STRICT AS
$func$
BEGIN                -- null is covered by STRICT
   IF a = '{}' THEN  -- empty
      a_1d = '{}';
      RETURN NEXT;
   ELSE              --  all other cases
      FOREACH a_1d SLICE 1 IN ARRAY a LOOP
         RETURN NEXT;
      END LOOP;
   END IF;
END
$func$
