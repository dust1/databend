select '=== Test limit ===';
select number from numbers_mt(100) order by number asc limit 10;
select '=== Test limit with offset ===';
select number from numbers_mt(100) order by number asc limit 10 offset 10;
