import * as hub from 'hub';

export class OmbBuoy {
  public dev: string = '';
  public sn: string = '';

  public latitude: number = undefined;
  public longitude: number = undefined;

  public iridium_lat: number = undefined;
  public iridium_lon: number = undefined;

  constructor(dev: string) {
    console.log("Omb: " + dev);
    this.dev = dev;
    this.sn = dev;
  }

  public hasGps(): boolean {
    return this.latitude !== undefined;
  }

  public async setLast() {
    const last = await hub.last_file(hub.API_CONF, this.dev);
    this.setPackage(last);
  }

  public lastContact(): Date | null {
    return null;
    // if (this.package) {
    //   return new Date(this.package.received * 1000.);
    // } else {
    //   return null;
    // }
  }

  public setPackage(p: any) {
    console.log(p);
    // this.package = p;

    // this.latitude = p.body.lat;
    // this.longitude = p.body.lon;

    // this.tower_lat = p.tower_lat;
    // this.tower_lon = p.tower_lon;
  }

  public any_lat = (): number => {
    return this.latitude;
  }

  public any_lon = (): number => {
    return this.longitude;
  }
}

