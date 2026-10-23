import { readFileSync, writeFileSync } from 'node:fs';

const f = 'D:/DOKIPON/tools/commerce-e2e/mock_data/contract-expectations.json';
let j = readFileSync(f, 'utf8');
const before = j;

j = j.replace(
  `    {
      "method": "GET",
      "path": "/v1/inventory",
      "expectStatus": 404,
      "note": "the inventory service exposes only the three reservation write routes; contracts/openapi.yaml records this as x-blocked-by: inventory-service-has-no-list-route on /admin/v1/inventory. This expectation flips to 200 when the listing route lands — that is the point of having it here."
    },`,
  `    {
      "method": "GET",
      "path": "/v1/inventory",
      "expectStatus": 200,
      "expectJson": "object",
      "note": "The inventory service gained a listing route, so the documented blocker (x-blocked-by: inventory-service-has-no-list-route on /admin/v1/inventory) is closed. The expectation was 404 while the route did not exist — that is the point of having it in the baseline."
    },`,
);

if (j === before) {
  console.error('no replacement applied');
  process.exit(1);
}
writeFileSync(f, j, 'utf8');
console.log('contract-expectations.json: /v1/inventory now 200');
