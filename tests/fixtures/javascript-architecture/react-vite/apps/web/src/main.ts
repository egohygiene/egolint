import fs from "node:fs";
import { admin } from "../../admin/src/main";
import { internalButton } from "../../../packages/ui/src/internal";

const missingPackage = import("package-that-does-not-exist");

export const web = [admin, internalButton, typeof fs.readFile, typeof missingPackage].join(
    ":",
);
