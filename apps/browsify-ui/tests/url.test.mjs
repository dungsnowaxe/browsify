import assert from 'node:assert/strict';
import { test } from 'node:test';
import { normalizeTestUrl } from '../src/utils/url.ts';

for (const [input, expected] of [
  ['github.com', 'https://github.com/'],
  ['  github.com/docs  ', 'https://github.com/docs'],
  ['https://example.com/path?q=one#section', 'https://example.com/path?q=one#section'],
  ['http://localhost:3000', 'http://localhost:3000/'],
  ['localhost:8080/test', 'https://localhost:8080/test'],
  ['custom-browser://open/item', 'custom-browser://open/item'],
  ['mailto:person@example.com', 'mailto:person@example.com'],
  ['tel:+123456789', 'tel:+123456789'],
  ['', null],
  ['   ', null],
  ['not a valid url', null],
  ['https://', null],
  ['https://not%20valid.example/', null],
  ['https://example.com/a b', 'https://example.com/a%20b'],
  ['http://[invalid', null],
  ['javascript:alert(1)', null],
]) {
  test(`normalizeTestUrl(${JSON.stringify(input)})`, () => {
    assert.equal(normalizeTestUrl(input), expected);
  });
}
