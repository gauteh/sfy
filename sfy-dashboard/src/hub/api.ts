export class ApiConf {
  public host: string;
  token: string;

  public constructor(host: string, token: string) {
    this.token = token;
    this.host = host;
  }

  public headers(): any {
    return {
      'headers': {
        'SFY_AUTH_TOKEN' : this.token,
      }
    }
  }

  public setToken(token: string) {
    this.token = token;
  }
}

export const SFY_SERVER='https://wavebug.met.no'

export let API_CONF: ApiConf = new ApiConf(SFY_SERVER, undefined);

