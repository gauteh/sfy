import { createRef, Component } from 'react';

import moment from 'moment';
import { Buoy, OmbBuoy } from 'models';
import * as hub from 'hub';

import { BuoyMap } from './BuoyMap';

import './BuoyIndex.scss';

interface Props {
  onLogout?: () => void;
}

interface State {
  buoys: Array<Buoy | OmbBuoy>;
  trackDev?: string;
  trackDays: number;
  mapSize: 1 | 2 | 3;
}

export class BuoyIndex
  extends Component<Props, State>
{

  public state: State = {
    buoys: [],
    trackDev: undefined,
    trackDays: 7,
    mapSize: 2,
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

  public showTrack = (buoy: any) => {
    this.setState({ trackDev: buoy.dev });
    this.bmap.current.showTrack(buoy, this.state.trackDays);
    this.focus(buoy);
  }

  public clearTrack = () => {
    this.setState({ trackDev: undefined });
    this.bmap.current.clearTrack();
  }

  public setMapSize = (size: 1 | 2 | 3) => {
    console.log('setMapSize:', size);
    this.setState({ mapSize: size });
    setTimeout(() => this.bmap.current?.invalidateSize(), 100);
  }

  public onDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const days = parseInt(e.target.value, 10);
    this.setState({ trackDays: days });
    // Reload track if one is currently shown.
    if (this.state.trackDev) {
      const buoy = this.state.buoys.find(b => b.dev === this.state.trackDev);
      if (buoy) {
        this.bmap.current.showTrack(buoy, days);
      }
    }
  }

  public Row = (buoy: any) => {
    const isTracked = buoy.dev === this.state.trackDev;
    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}
        style={isTracked ? { fontWeight: 'bold' } : {}}>
        <td>
          <a href="#" title={buoy.dev} onClick={() => this.showTrack(buoy)}>{buoy.sn}</a>
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
    const mapHeights: Record<1 | 2 | 3, string> = { 1: '33dvh', 2: '67dvh', 3: '' };
    const mapStyle: React.CSSProperties = this.state.mapSize === 3
      ? { flex: '1 1 auto', minHeight: 0, transition: 'flex 0.2s' }
      : { flex: `0 0 ${mapHeights[this.state.mapSize]}`, minHeight: 0, transition: 'flex 0.2s' };

    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100dvh', overflow: 'hidden' }}>

        <div style={mapStyle}>
          <BuoyMap buoys={this.state.buoys} ref={this.bmap} onBuoyClick={this.showTrack} />
        </div>

        {this.state.mapSize < 3 &&
          <div style={{ flex: '1 1 auto', overflowY: 'auto', minHeight: 0 }}>
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
        }

        <div className="no-margin flex-shrink-0 border-top bg-light px-2">
          <div className="d-flex align-items-center gap-2 py-1 flex-wrap">
            <span className="text-muted small">Track:</span>
            <select className="form-select form-select-sm w-auto" value={this.state.trackDays} onChange={this.onDaysChange}>
              <option value={1}>1 day</option>
              <option value={7}>7 days</option>
              <option value={30}>30 days</option>
              <option value={90}>90 days</option>
            </select>
            {this.state.trackDev &&
              <button className="btn btn-sm btn-outline-secondary" onClick={this.clearTrack}>✕ Clear track</button>
            }
            {this.state.trackDev &&
              <span className="text-muted small">{this.state.trackDev}</span>
            }

            <span className="text-muted small ms-auto" title="Map size">🗺</span>
            <div className="btn-group btn-group-sm" role="group" aria-label="Map size">
              {([1, 2, 3] as const).map(s => (
                <button
                  key={s}
                  type="button"
                  className={`btn ${this.state.mapSize === s ? 'btn-secondary' : 'btn-outline-secondary'}`}
                  onClick={() => this.setMapSize(s)}
                  title={`${s}/3 map`}
                >
                  {s}/3
                </button>
              ))}
            </div>

            {this.props.onLogout &&
              <button type="button" className="btn btn-outline-dark btn-sm" onClick={this.props.onLogout}>✕ Log out</button>
            }
            <a href="https://github.com/gauteh/sfy" className="text-muted small">sfy</a>
          </div>
        </div>

      </div>
    );
  }
}
