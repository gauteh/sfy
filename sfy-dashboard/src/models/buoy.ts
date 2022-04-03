import * as hub from 'hub';

export class Buoy {
  public dev: string = '';
  public sn: string = '';
  public files: string[];

  public latitude: number;
  public longitude: number;

  public tower_lat: number;
  public tower_lon: number;

  public package: any;

  constructor(dev: string, files: string[]) {
    this.dev = dev;
    this.files = files;

    // Sort in ascending order by time received.
    this.files.sort((a, b) => {
      let a1 = parseInt(a.split('-')[0]);
      let b1 = parseInt(b.split('-')[0]);

      return b1 - a1
    });
  }

  public hasGps(): boolean {
    return this.latitude !== undefined;
  }

  public lastFile(filter: string | undefined) {
    let files = this.files.filter((fname: string) => filter !== undefined ? fname.includes(filter) : true);
    if (files.length >= 1) {
      return files[files.length - 1];
    } else {
      return null;
    }
  }

  public lastContact(): Date | null {
    if (this.files.length > 0) {
      return new Date(parseInt(this.files[this.files.length - 1].split('-')[0]));
    } else {
      return null;
    }
  }

  public setPackage(p: any) {
    this.package = p;

    this.sn = p.sn;

    this.latitude = p.body.longitude;
    this.longitude = p.body.longitude;

    this.tower_lat = p.tower_lat;
    this.tower_lon = p.tower_lon;
  }

  public any_lat(): number {
    return this.latitude != undefined ? this.latitude : this.tower_lat;
  }

  public any_lon(): number {
    return this.longitude != undefined ? this.longitude : this.tower_lon;
  }
}
