export class ApiConf {
  public host: string | undefined;
  token: string | undefined;

  public constructor(host: string | undefined, token: string | undefined) {
    this.token = token;
    this.host = host;
  }

  public headers(): any {
    return {
      'headers': {
        'SFY_AUTH_TOKEN': this.token,
      }
    }
  }

  public setToken(token: string | undefined) {
    this.token = token;
  }
}

export const SFY_SERVER = process.env.SFY_SERVER;

export let API_CONF: ApiConf = new ApiConf(SFY_SERVER, undefined);

