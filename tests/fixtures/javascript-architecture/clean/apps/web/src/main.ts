import { publint } from "publint";
import { formatMessage } from "publint/utils";

import { publicValue } from "../../../packages/ui/src/index";

export const web = `${publicValue}:${typeof publint}:${typeof formatMessage}`;
