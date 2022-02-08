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

  constructor(props, context) {
    super(props, context);
  }

  componentDidMount() {
    this.loadBuoys();
  }

  public loadBuoys = () => {
    this.state.buoys.length = 0;
    this.setState({buoys: []});

    hub.get_buoys(hub.API_CONF).pipe(
      mergeMap(buoys => buoys),
      concatMap(b => hub.get_buoy(hub.API_CONF, b)),
      concatMap(b => {
        let last = b.files.reverse().find((fname) => fname.endsWith("axl.qo.json"));
        console.log("getting files for: " + b.dev + ", last: " + last);

        if (last !== undefined) {
          return hub.get_file(hub.API_CONF, b.dev, last).pipe(
            map(f => {
              b.setPackage(f);
              return b;
            })
          );
        } else {
          return of(b);
        }
      })
    ).subscribe(b => {
      this.state.buoys.push(b);
      this.state.buoys.sort((a, b) => b.lastContact().getTime() - a.lastContact().getTime());
      this.setState({buoys: this.state.buoys});
    }
    );
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
          {buoy.latitude !== undefined ? '🛰' : '📡'}
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
