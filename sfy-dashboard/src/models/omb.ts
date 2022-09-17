import * as hub from 'hub';

export class OmbBuoy {
  public dev: string = '';
  public sn: string = '';

  public latitude: number = undefined;
  public longitude: number = undefined;

  public iridium_lat: number = undefined;
  public iridium_lon: number = undefined;

  public package = undefined;

  constructor(dev: string, last: any) {
    console.log("Omb: " + dev);
    this.dev = dev;
    this.sn = dev;

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
      return new Date(this.package.datetime);
    } else {
      return null;
    }
  }

  public setPackage(p: any) {
    this.package = p;

    this.iridium_lat = p.body.iridium_pos.lat;
    this.iridium_lon = p.body.iridium_pos.lon;

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

  public formatted_position(): string {
    if (this.any_lat() != null) {
      return `${this.any_lat().toFixed(9)},${this.any_lon().toFixed(9)}`;
    } else {
      return "";
    }
  }
}

