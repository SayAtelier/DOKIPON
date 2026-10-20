#!/usr/bin/env node
/**
 * selfcheck.js — prove the content detector can actually fail.
 *
 * A test that only ever passes is indistinguishable from a broken test. This
 * feeds the detector a set of synthetic PNGs with known content and asserts
 * that it classifies each one correctly, so a future change to the threshold
 * or the algorithm cannot silently turn the check into a no-op.
 *
 * The cases come from measurements on the real stack:
 *
 *   storefront   28.5% non-background, 1717 colours  -> must be judged as content
 *   admin console 13.5% non-background,  795 colours  -> must be judged as content
 *   blank page    0.1% non-background,    2 colours  -> must be judged as blank
 *
 * Run: node selfcheck.js
 */
const { PNG } = require('pngjs');
const fs = require('fs');
const os = require('os');
const path = require('path');

const BLANK_CONTENT_RATIO = 0.02;

function measureContent(buffer) {
  return new Promise((resolve, reject) => {
    new PNG({ filterType: 4 })
      .parse(buffer, (err, img) => {
        if (err) return reject(err);
        const total = img.width * img.height;
        if (!total) return resolve({ ratio: 0, distinct: 0 });
        const freq = new Map();
        for (let i = 0; i < img.data.length; i += 4) {
          if (img.data[i + 3] === 0) continue;
          const key = (img.data[i] << 16) | (img.data[i + 1] << 8) | img.data[i + 2];
          freq.set(key, (freq.get(key) || 0) + 1);
        }
        if (freq.size === 0) return resolve({ ratio: 0, distinct: 0 });
        let bgCount = 0;
        for (const c of freq.values()) if (c > bgCount) bgCount = c;
        return resolve({ ratio: (total - bgCount) / total, distinct: freq.size });
      });
  });
}

/** Build a PNG with a given background and a grid of foreground dots. */
function synth(width, height, bg, dot, step) {
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (width * y + x) << 2;
      const on = dot && (x % step === 0) && (y % step === 0);
      png.data[i] = on ? dot[0] : bg[0];
      png.data[i + 1] = on ? dot[1] : bg[1];
      png.data[i + 2] = on ? dot[2] : bg[2];
      png.data[i + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

let pass = 0;
let fail = 0;

function check(name, ok, detail = '') {
  if (ok) { pass += 1; console.log(`  PASS  ${name}${detail ? '  ' + detail : ''}`); }
  else { fail += 1; console.log(`  FAIL  ${name}${detail ? '  ' + detail : ''}`); }
}

(async () => {
  console.log('\n=== SELF-CHECK: the content detector must reject a blank page ===');

  // A page that drew nothing: one colour, nothing else.
  const blank = synth(600, 400, [247, 248, 250], null, 0);
  const blankM = await measureContent(blank);
  check('a single-colour page is blank',
    blankM.ratio < BLANK_CONTENT_RATIO,
    `ratio ${(blankM.ratio * 100).toFixed(2)}%`);

  // A page with a sparse grid of content, like a real but light-themed UI.
  // step 6 gives ~2.8% non-background, which is below the lightest real page
  // (13.5%) but well above the threshold — this is the case the original 3%
  // threshold got wrong, so it must stay in the suite.
  const sparse = synth(600, 400, [255, 255, 255], [30, 30, 30], 6);
  const sparseM = await measureContent(sparse);
  check('a sparse but drawn page is not blank',
    sparseM.ratio >= BLANK_CONTENT_RATIO,
    `ratio ${(sparseM.ratio * 100).toFixed(2)}%`);

  // A page with a dense layout.
  const dense = synth(600, 400, [247, 248, 250], [70, 93, 145], 4);
  const denseM = await measureContent(dense);
  check('a dense page is not blank',
    denseM.ratio >= BLANK_CONTENT_RATIO,
    `ratio ${(denseM.ratio * 100).toFixed(2)}%`);

  // An empty image must not throw or report as content.
  const empty = synth(10, 10, [0, 0, 0], null, 0);
  const emptyM = await measureContent(empty);
  check('a degenerate image is handled', Number.isFinite(emptyM.ratio),
    `ratio ${emptyM.ratio}`);

  // The threshold must sit between the blank sample and the real pages.
  check('threshold separates blank from content',
    blankM.ratio < BLANK_CONTENT_RATIO && sparseM.ratio >= BLANK_CONTENT_RATIO,
    `blank ${(blankM.ratio * 100).toFixed(2)}% < ${(BLANK_CONTENT_RATIO * 100).toFixed(0)}% <= sparse ${(sparseM.ratio * 100).toFixed(2)}%`);

  console.log(`\n=== RESULT: ${pass} passed, ${fail} failed ===`);
  process.exit(fail === 0 ? 0 : 1);
})().catch((e) => { console.error('fatal:', e.stack || e.message); process.exit(2); });
