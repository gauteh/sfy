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

// In development the client always uses relative URLs so requests go through
// the webpack dev server proxy (avoiding CORS). In production the full server
// URL baked in at build time is used.
export const SFY_SERVER = process.env.NODE_ENV === 'development'
  ? ''
  : process.env.SFY_SERVER;

export let API_CONF: ApiConf = new ApiConf(SFY_SERVER, undefined);

