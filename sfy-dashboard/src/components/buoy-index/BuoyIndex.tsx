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
        console.log("getting files for: " + b.dev);
        let last = b.files.reverse().find((fname) => fname.endsWith("axl.qo.json"));

        return hub.get_file(hub.API_CONF, b.dev, last).pipe(
          map(f => {
            b.setPackage(f);
            return b;
          })
        );
      })
    ).subscribe(b => {
      this.state.buoys.push(b);
      this.setState({buoys: this.state.buoys});
    }
    );
  }

  public Row(buoy) {
    const formatDate = (date: number): JSX.Element => {
      return (<span>{moment(new Date(date)).fromNow()} - {moment(date).format("YYYY-MM-DD hh:mm:ss")} UTC</span>);
    };

    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}>
        <td>
          {buoy.dev}
        </td>
        <td>
          { buoy.any_lat().toFixed(5) }
        </td>
        <td>
          { buoy.any_lon().toFixed(5) }
        </td>
        <td>
          { buoy.latitude != undefined ? '🛰' : '📡' }
        </td>
        <td>
          {formatDate(buoy.lastContact())}
        </td>
      </tr>
    );
  }

  public render() {
    return (
      <div>
        <BuoyMap buoys={this.state.buoys} />

        <div class="container-fluid">
          <table class="ti table table-dark table-striped">
            <thead>
              <th scope="col">Device</th>
              <th scope="col">Latitude</th>
              <th scope="col">Longitude</th>
              <th scope="col">Position</th>
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
