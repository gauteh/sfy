import {Component} from 'inferno';

import moment from 'moment';
import cx from 'classnames';

import {of} from 'rxjs';
import {finalize, tap, concatMap, mergeMap, switchMap, map} from 'rxjs/operators';
import {Buoy} from 'models';
import * as hub from 'hub';

import {BuoyMap} from './BuoyMap';

import './BuoyIndex.scss';

interface Props {
}

interface State {
  buoys: Buoy[];
}

export class BuoyIndex
  extends Component<Props, State>
{

  public state = {
    buoys: new Array<Buoy>(),
  };

  constructor(props: Props, context: any) {
    super(props, context);
  }

  componentDidMount() {
    (async () => await this.loadBuoys())();
  }

  public loadBuoys = async () => {
    this.state.buoys.length = 0;
    this.setState({buoys: []});

    const devs = await hub.get_buoys(hub.API_CONF);
    for (const devsn of devs) {
      if (devsn[0] !== "lost+found") {
        let b = new Buoy(devsn[0], devsn[1]);
        await b.setLast();

        this.state.buoys.push(b);
        this.state.buoys.sort((a, b) => b.lastContact().getTime() - a.lastContact().getTime());
        this.setState({buoys: this.state.buoys});
      }
    }
  }

  public Row(buoy) {
    const formatDate = (date: number): JSX.Element => {
      return (<span> - </span>);
    };

    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}>
        <td>
          <span title={buoy.dev}>{buoy.sn}</span>
        </td>
        <td>
          {buoy.any_lat().toFixed(9)}
        </td>
        <td>
          {buoy.any_lon().toFixed(9)}
        </td>
        <td>
          {buoy.hasGps() ? 'GPS' : 'Cel.'}
        </td>
        <td>
          <span title={moment(buoy.lastContact()).utc().format("YYYY-MM-DD hh:mm:ss") + " UTC"}>
            {moment(new Date(buoy.lastContact())).fromNow()}
          </span>
        </td>
      </tr>
    );
  }

  public render() {
    return (
      <div>
        <BuoyMap buoys={this.state.buoys} />

        <div class="container-fluid no-margin">
          <table class="ti table table-striped">
            <thead>
              <th scope="col">Device</th>
              <th scope="col">Latitude (°N)</th>
              <th scope="col">Longitude (°E)</th>
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
