# Project plan and notes

There are several ways to get this project working. The most scalable
would be to have an endpoint (with authentication), accepting updates
from scrapers, so that scrapers could be hosted anywhere, and
implemented in any language. This would make it a lot easier in the
long run to have a large selection of sites and restaurant menus,
as others could contribute in any language they prefer.

But, to keep the scope small enough that I might actually get it
done any time on my spare time, while still leaving room for upgrading
to the solution above, I think I'll do something like the following:

- Create one HTTP server binary for serving GET requests only.
  - This will serve responses as JSON
  - HTML output via templates could be added to this, or it could
    be implemented as a separate frontend on top of this, in whatever
    language and framework desirable
- Create one scraper binary responsible for keeping the DB updated
  with fresh results
  - The top level will be responsible for scheduling, sending signals
    to registered sub-scrapers at the desired interval, and then upon
    receiving the results, updating the DB.
  - Have a structure of sub-scrapers that communicate with the top
    level scheduler via channels.
  - Each sub-scraper should be responsible for producing a restaurant
    with its current dishes. It should still be possible though, to
    have the same scraper produce results for many restaurants, since
    the case I'm starting from, is scraping a single page that contains
    the menus for all restaurants at a certain physical site.
  - To enable the previous point, a scrape result should contain
    references to where within the overall structure the result should
    be saved, i.e. country/city/site.
  - The structure of country/city/site should be statically/manually
    defined, and updated only when scrapers are added or removed.
  - When a result for a restaurant is received, any previously
    stored restaurant with the same ID should be overwritten, and all
    dishes for this restaurant cleared out before adding the dishes
    from the new result. This to make sure we don't have any stale
    dishes from earlier results. The most easy way to do this, is
    likely to have a "on cascade delete" reference from dish to
    restaurant, and then delete and insert the restaurant from each
    result, before inserting its dishes. This might be poor practice
    from a DB efficiency point of view, but DB performance is not the
    priority at this point of the project.

Actually, when mentioning separate binaries above, it can still all
be in a single binary, with different modes triggered via subcommands,
like git.
