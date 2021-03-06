import * as hub from 'hub';

export class OmbBuoy {
  public dev: string = '';
  public sn: string = '';

  public latitude: number = undefined;
  public longitude: number = undefined;

  public iridium_lat: number = undefined;
  public iridium_lon: number = undefined;

  public package = undefined;

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
    if (this.package) {
      return new Date(this.package.datetime);
    } else {
      return null;
    }
  }

  public setPackage(p: any) {
    this.package = p;

    this.iridium_lat = p.body.iridium_pos.lat;
    this.iridium_lon = p.body.iridium_pos.lon;

    console.log(p);
    if (p.type === "gps" && p.body.messages.length > 0) {
      let pos = p.body.messages[p.body.messages.length - 1];
      this.latitude = pos.latitude;
      this.longitude = pos.longitude;
    }
  }

  public any_lat = (): number => {
    return this.latitude !== undefined ? this.latitude : this.iridium_lat;
  }

  public any_lon = (): number => {
    return this.longitude !== undefined ? this.longitude : this.iridium_lon;
  }
}

