const {readFileSync,writeFileSync}=require('fs');
const f='D:/DOKIPON/tools/commerce-e2e/mock_data/contract-expectations.json';
let j=readFileSync(f,'utf8');
const before=j;
// The /v1/inventory entry: flip the expectation now that the listing route
// exists (the contract no longer marks it x-implemented: false).
j=j.replace(/(\{[^}]*?"path": "\/v1\/inventory"[^}]*?"expectStatus": )404/, '$1200');
if(j===before){ console.error('no replacement applied'); process.exit(1); }
writeFileSync(f,j,'utf8');
const d=JSON.parse(j);
const e=d.paths.find(p=>p.path==='/v1/inventory');
console.log('/v1/inventory expectStatus =', e.expectStatus);
