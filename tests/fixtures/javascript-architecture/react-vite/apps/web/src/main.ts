import fs from "node:fs";
import { admin } from "../../admin/src/main";
import { internalButton } from "../../../../packages/ui/src/internal";

export const web = `${admin}:${internalButton}:${typeof fs.readFile}`;
