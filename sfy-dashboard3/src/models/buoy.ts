import * as hub from 'hub';

export class Buoy {
  public dev: string = '';
  public sn: string = '';

  public latitude: number | undefined;
  public longitude: number | undefined;

  public tower_lat: number | undefined;
  public tower_lon: number | undefined;

  public package: any;

  constructor(dev: string, sn: string, last: any) {
    console.log("SFY: " + dev + ", " + sn);
    this.dev = dev;
    this.sn = sn;

    try {
      this.setPackage(JSON.parse(atob(last)));
    } catch(err) {
      console.log("failed to load buoy: " + dev + ":" + err);
    }
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

  public any_lat(): number | undefined {
    return this.latitude !== undefined ? this.latitude : this.tower_lat;
  }

  public any_lon(): number | undefined {
    return this.longitude !== undefined ? this.longitude : this.tower_lon;
  }
}
