-- siphash
SELECT SIPHASH('1234567890');
SELECT SIPHASH(1);
SELECT SIPHASH(1.2);

-- siphash64 (alias)
SELECT SIPHASH64('1234567890');
SELECT SIPHASH64(1);
SELECT SIPHASH64(1.2);

-- blake3
SELECT BLAKE3('1234567890');
SELECT BLAKE3('1');
SELECT BLAKE3('1.2');

-- md5
SELECT MD5('1234567890');
SELECT MD5('1');
SELECT MD5('1.2');

-- SHA1
SELECT SHA1('1234567890');
SELECT SHA1('1');
SELECT SHA1('1.2');