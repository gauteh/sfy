export class ApiConf {
  public host: string;
  token: string;

  public constructor(host: string, token: string) {
    if (host.length == 0 || token.length == 0) {
      throw new Error("Empty host and token.");
    }

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
}
