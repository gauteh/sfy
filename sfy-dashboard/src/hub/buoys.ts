import {ApiConf} from './api';

import {from, Observable} from 'rxjs';
import {Buoy} from 'models';

export async function get_buoys(api: ApiConf): Promise<string[][]> {
  return fetch(api.host + '/buoys', api.headers())
    .then((response) => {
      if (response.ok) {
        return response.json() as Promise<string[][]>;
      } else {
        throw new Error("not ok");
      }
    });
}

export async function get_buoy(api: ApiConf, dev: string, sn: string): Promise<Buoy> {
  return fetch(api.host + '/buoys/' + dev, api.headers())
    .then(async (response) => {
      if (response.ok) {
        const files = await response.json();
        return new Buoy(dev, sn, files);
      } else {
        throw new Error("not ok");
      }
    });
}

export async function last_file(api: ApiConf, dev: string): Promise<any> {
  const response = await fetch(api.host + '/buoys/' + dev + '/last', api.headers());
  if (response.ok) {
    return response.json() as Promise<any>;
  }
  else {
    throw new Error("not ok");
  }
}

export function get_file(api: ApiConf, dev: string, file: string): Observable<any> {
  return from(fetch(api.host + '/buoys/' + dev + '/' + file, api.headers())
    .then((response) => {
      if (response.ok) {
        return response.json() as Promise<any>;
      } else {
        throw new Error("not ok");
      }
    }));
}
