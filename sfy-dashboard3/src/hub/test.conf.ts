import {ApiConf} from "./api";

export const TEST_CONF: ApiConf = new ApiConf(process.env.SFY_SERVER, process.env.SFY_READ_TOKEN);
