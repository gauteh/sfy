import {ApiConf} from './api';

import {from, Observable} from 'rxjs';
import {Buoy} from 'models';

export function get_buoys(api: ApiConf): Observable<string[]> {
  return from(fetch(api.host + '/buoys', api.headers())
    .then((response) => {
      if (response.ok) {
        return response.json() as Promise<string[]>;
      } else {
        throw new Error("not ok");
      }
    }));
}

export function get_buoy(api: ApiConf, dev: string): Observable<Buoy> {
  return from(fetch(api.host + '/buoys/' + dev, api.headers())
    .then((response) => {
      if (response.ok) {
        return response.json().then((files) => {
          return new Buoy(dev, files);
        });
      } else {
        throw new Error("not ok");
      }
    }));
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
