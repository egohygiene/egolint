import assert from "node:assert/strict";
import test from "node:test";

import { buttonLabel } from "./dist/index.js";

test("published entrypoint remains executable", () => {
    assert.equal(buttonLabel("ego"), "ego");
});
