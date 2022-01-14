export class Buoy {
  public dev: string = '';
  public files: string[];

  constructor(dev: string, files: string[]) {
    this.dev = dev;
    this.files = files;
  }
}

