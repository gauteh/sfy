import {linkEvent, createRef, Component} from 'react';

import moment from 'moment';

import {Buoy, OmbBuoy} from '/src/models';
import * as hub from '/src/hub';

import {BuoyMap} from './BuoyMap';

import './BuoyIndex.scss';

interface Props {
}

interface State {
  buoys: Array<Buoy|OmbBuoy>;
}

export class BuoyIndex
  extends Component<Props, State>
{

  public state = {
    buoys: new Array<Buoy | OmbBuoy>(),
  };

  public bmap: any;

  constructor(props: Props, context: any) {
    super(props, context);

    this.bmap = createRef();
  }

  componentDidMount() {
    (async () => await this.loadBuoys())();
  }

  public loadBuoys = async () => {
    const devs = await hub.get_buoys(hub.API_CONF);
    const buoys = devs.map(devsn => {
      if (devsn[0] !== "lost+found") {
        let b = undefined;

        if (devsn[2] === "sfy") {
          b = new Buoy(devsn[0], devsn[1], devsn[3]);
        } else if (devsn[2] === "omb") {
          b = new OmbBuoy(devsn[0], devsn[3]);
        } else {
          console.log("Unknown buoy:" + devsn);
        }

        return b;
      } else {
        return undefined;
      }
      }).filter(b => b !== undefined);

    buoys.sort((a, b) => b.lastContact().getTime() - a.lastContact().getTime());
    this.state.buoys = buoys;
    this.setState({buoys: this.state.buoys});
  }

  public Row = (buoy) => {
    const formatDate = (date: number): JSX.Element => {
      return (<span> - </span>);
    };

    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}>
        <td>
          <a href="#" title={buoy.dev} onClick={linkEvent(buoy, this.focus)}>{buoy.sn}</a>
        </td>
        <td>
          <a href="#" title="Copy to clipboard" onClick={ linkEvent(buoy, this.copyPosition) }>
            {buoy.any_lat().toFixed(9)},{buoy.any_lon().toFixed(9)}
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

  public focus = (buoy) => {
    this.bmap.current.focus(buoy);
  }

  public copyPosition = (buoy) => {
    const position = `${buoy.any_lat().toFixed(9)},${buoy.any_lon().toFixed(9)}`;
    navigator.clipboard.writeText(position);
  }

  public render() {
    return (
      <div>
        <BuoyMap buoys={this.state.buoys} ref={this.bmap}/>

        <div class="container-fluid no-margin">
          <table class="ti table table-striped">
            <thead>
              <th scope="col">Device</th>
              <th scope="col">Latitude (°N), Longitude (°E)</th>
              <th scope="col">Source</th>
              <th scope="col">Last contact</th>
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

