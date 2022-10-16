import { createRef, Component } from 'react';

import moment from 'moment';
import { Buoy, OmbBuoy } from 'models';
import * as hub from 'hub';

import { BuoyMap } from './BuoyMap';

import './BuoyIndex.scss';

interface Props {
}

interface State {
  buoys: Array<Buoy | OmbBuoy>;
}

export class BuoyIndex
  extends Component<Props, State>
{

  public state: State = {
    buoys: [],
  };

  public bmap: any;
  loaded = false;

  constructor(props: Props) {
    super(props);

    this.bmap = createRef();
  }

  async componentDidMount() {
    if (!this.loaded) {
      this.loaded = true;
      await this.loadBuoys();
    }
  }

  public async loadBuoys() {
    const devs = await hub.get_buoys(hub.API_CONF);
    const buoys: Array<OmbBuoy | Buoy> = devs
    .filter(devsn => devsn[0] !== "lost+found" && (devsn[2] === "sfy" || devsn[2] === "omb"))
    .map(devsn => {
        if (devsn[2] === "sfy") {
          return new Buoy(devsn[0], devsn[1], devsn[3]);
        } else if (devsn[2] === "omb") {
          return new OmbBuoy(devsn[0], devsn[3]);
        }
        }) as Array<OmbBuoy | Buoy>;

    buoys.sort((a, b) => (b.lastContact()?.getTime() || 0) - (a.lastContact()?.getTime() || 0));
    this.state.buoys = buoys;
    this.setState({ buoys: this.state.buoys });
  }

  public Row = (buoy: any) => {
    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}>
        <td>
          <a href="#" title={buoy.dev} onClick={() => this.focus(buoy)}>{buoy.sn}</a>
        </td>
        <td>
          <a href="#" title="Copy to clipboard" onClick={() => this.copyPosition(buoy)}>
            {buoy.formatted_position()}
          </a>
        </td>
        <td>
          {buoy.hasGps() ? 'GPS' : 'Cel/Ird'}
        </td>
        <td>
          <span title={moment(buoy.lastContact()).utc().format("YYYY-MM-DD hh:mm:ss") + " UTC"}>
            {moment(new Date(buoy.lastContact())).fromNow()}
          </span>
        </td>
      </tr>
    );
  }

  public focus = (buoy: any) => {
    this.bmap.current.focus(buoy);
  }

  public copyPosition = (buoy: any) => {
    const position = `${buoy.any_lat().toFixed(9)},${buoy.any_lon().toFixed(9)}`;
    navigator.clipboard.writeText(position);
  }

  public render() {
    return (
      <div>
        <BuoyMap buoys={this.state.buoys} ref={this.bmap} />

        <div className="container-fluid no-margin">
          <table className="ti table table-striped">
            <thead>
              <tr>
                <th scope="col">Device</th>
                <th scope="col">Latitude (°N), Longitude (°E)</th>
                <th scope="col">Source</th>
                <th scope="col">Last contact</th>
              </tr>
            </thead>
            <tbody>
              {this.state.buoys.map(this.Row)}
            </tbody>
          </table>
        </div>
      </div>
    );
  }
}
