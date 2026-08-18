import test from "node:test";
import assert from "node:assert/strict";
import { clockAngles } from "../src/clock.js";

const at = (hours, minutes, seconds = 0) => new Date(2026, 0, 1, hours, minutes, seconds, 0);

test("clock angles cover cardinal times and interpolate the hour hand", () => {
  assert.deepEqual(clockAngles(at(12, 0)), { hour: 0, minute: 0, second: 0 });
  assert.deepEqual(clockAngles(at(3, 15)), { hour: 97.5, minute: 90, second: 0 });
  assert.deepEqual(clockAngles(at(6, 30)), { hour: 195, minute: 180, second: 0 });
  assert.deepEqual(clockAngles(at(23, 59)), { hour: 359.5, minute: 354, second: 0 });
});
