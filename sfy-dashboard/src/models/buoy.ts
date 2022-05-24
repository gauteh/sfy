import * as hub from 'hub';

export class Buoy {
  public dev: string = '';
  public sn: string = '';

  public latitude: number;
  public longitude: number;

  public tower_lat: number;
  public tower_lon: number;

  public package: any;

  constructor(dev: string, sn: string) {
    this.dev = dev;
    this.sn = sn;
  }

  public hasGps(): boolean {
    return this.latitude !== undefined;
  }

  public async setLast() {
    const last = await hub.last_file(hub.API_CONF, this.dev);
    this.setPackage(last);
  }

  public lastContact(): Date | null {
    if (this.package) {
      return new Date(this.package.received * 1000.);
    } else {
      return null;
    }
  }

  public setPackage(p: any) {
    this.package = p;

    this.latitude = p.body.lat;
    this.longitude = p.body.lon;

    this.tower_lat = p.tower_lat;
    this.tower_lon = p.tower_lon;
  }

  public any_lat(): number {
    return this.latitude !== undefined ? this.latitude : this.tower_lat;
  }

  public any_lon(): number {
    return this.longitude !== undefined ? this.longitude : this.tower_lon;
  }
}
