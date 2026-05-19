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
      if (last && last.length > 0) {
        this.setPackage(JSON.parse(atob(last)));
      }
    } catch(err) {
      console.log("failed to load buoy: " + dev + ":" + err);
    }
  }

  public hasGps(): boolean {
    if (this.package != null) {
      // SFY4 GPS files: GPS is in body, not the notecard location system
      if (this.package.file === 'egpsb.qo') {
        return this.package.body?.lat != null;
      }
      if (this.package.file === 'axlb.qo') {
        return this.package.body?.position_time != null;
      }
      return this.package.best_location_type === 'gps';
    } else {
      return false;
    }
  }

  public async setLast() {
    const last = await hub.last_file(hub.API_CONF, this.dev);
    this.setPackage(last);
  }

  public lastContact(): Date | null {
    if (this.package) {
      return new Date(this.package.received * 1000.);
    } else {
      return new Date(0);
    }
  }

  public setPackage(p: any) {
    this.package = p;

    if (p.file === 'egpsb.qo') {
      // lat/lon are integers scaled by 1e7
      this.latitude = p.body?.lat != null ? p.body.lat / 1e7 : undefined;
      this.longitude = p.body?.lon != null ? p.body.lon / 1e7 : undefined;
    } else {
      // axlb.qo, axl.qo, _track.qo: lat/lon already in degrees
      this.latitude = p.body?.lat;
      this.longitude = p.body?.lon;
    }

    this.tower_lat = p.tower_lat;
    this.tower_lon = p.tower_lon;
  }

  public position_time(): Date | undefined {
    if (this.package != null) {
      if (this.package.file === 'egpsb.qo') {
        return this.package.body?.timestamp != null
          ? new Date(this.package.body.timestamp)
          : undefined;
      }
      if (this.package.file === 'axlb.qo') {
        return this.package.body?.position_time != null
          ? new Date(this.package.body.position_time * 1000)
          : undefined;
      }
      return new Date(this.package.best_location_when * 1000.);
    } else {
      return undefined;
    }
  }

  public any_lat(): number | undefined {
    if (this.package != null) {
      // For SFY4 files, prefer GPS-derived body position over tower estimate
      if (this.package.file === 'egpsb.qo' || this.package.file === 'axlb.qo') {
        return this.latitude ?? this.package.best_lat;
      }
      return this.package.best_lat;
    } else {
      return undefined;
    }
  }

  public any_lon(): number | undefined {
    if (this.package != null) {
      // For SFY4 files, prefer GPS-derived body position over tower estimate
      if (this.package.file === 'egpsb.qo' || this.package.file === 'axlb.qo') {
        return this.longitude ?? this.package.best_lon;
      }
      return this.package.best_lon;
    } else {
      return undefined;
    }
  }

  public formatted_position(): string {
    if (this.any_lat() != null) {
      return `${this.any_lat()?.toFixed(9)},${this.any_lon()?.toFixed(9)}`;
    } else {
      return "";
    }
  }
}
