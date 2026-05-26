// Kosmocrates Node binding — minimal end-to-end example.
//
// Run:
//   1. Build the WASM package once:
//        wasm-pack build ../../crates/pse-wasm --target nodejs --release \
//          --out-dir ../../bindings/node/pkg --scope kosmocrates
//   2. Run this script:
//        node examples/hello.mjs
//
// Once @kosmocrates/pse-wasm-node is on npm, replace the relative
// import with: `import { PseWasm } from '@kosmocrates/pse-wasm-node';`
import { PseWasm } from '../pkg/pse_wasm.js';

const csv = `timestamp,sensor,value
0,s1,1.0
1,s1,1.1
2,s1,1.05
3,s1,1.0
4,s2,7.5
5,s2,7.4
6,s2,7.6
7,s2,7.5
`;

const pse = new PseWasm();

console.log('--- ingest ---');
console.log(JSON.parse(pse.ingest_csv(csv)));

console.log('\n--- run 200 ticks ---');
console.log(JSON.parse(pse.run(200)));

console.log('\n--- status ---');
console.log(JSON.parse(pse.status()));

const crystals = JSON.parse(pse.crystals());
console.log(`\n--- crystals (${crystals.length} total) ---`);
console.log(crystals.slice(0, 3));
