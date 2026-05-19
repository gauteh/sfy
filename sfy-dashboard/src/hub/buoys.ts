import {ApiConf} from './api';

import {from, Observable} from 'rxjs';
import {Buoy} from 'models';

export async function get_buoys(api: ApiConf): Promise<string[][]> {
  return fetch(api.host + '/buoys/', api.headers())
    .then((response) => {
      if (response.ok) {
        return response.json() as Promise<string[][]>;
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

export interface TrackPoint {
  t: number;
  lat: number;
  lon: number;
}

export async function buoy_track(api: ApiConf, dev: string, from: number, to: number): Promise<TrackPoint[]> {
  const response = await fetch(`${api.host}/buoys/${dev}/track/from/${from}/to/${to}`, api.headers());
  if (response.ok) {
    return response.json() as Promise<TrackPoint[]>;
  } else {
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
