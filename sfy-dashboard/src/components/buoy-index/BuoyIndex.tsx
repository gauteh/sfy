import {Component} from 'inferno';

import moment from 'moment';
import cx from 'classnames';

import {of} from 'rxjs';
import {finalize, tap, mergeMap, switchMap, map} from 'rxjs/operators';
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
    console.log('loading buoys..');
    this.state.buoys.length = 0;
    this.setState({buoys: []});

    hub.get_buoys(hub.API_CONF).pipe(
      mergeMap(buoys => buoys),
      switchMap(b => hub.get_buoy(hub.API_CONF, b)),
    ).subscribe(b => {
      console.log("adding buoy", b);
      this.state.buoys.push(b);
      this.setState({buoys: this.state.buoys});
    }
    );
  }

  public Row(buoy) {

    const formatDate = (date: number): JSX.Element => {
      return (<span>{moment(new Date(date * 1000)).fromNow()}</span>);
    };

    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}>
        <td class="ti-dev">
          buoy: {buoy.dev}
        </td>
        <td class="ti-authors">
        </td>
      </tr>
    );
  }

  public Rows(props) {
    const buoys = props.buoys;

    return (
      buoys.map((buoy) =>
        this.Row({
          thread: buoy
        }))
    );
  }

  public render() {
    return (
      <div class="container-fluid">
        <BuoyMap buoys={this.state.buoys} />

        <table class="ti table table-dark table-borderless table-sm">
          <tbody>
            {this.state.buoys.map(this.Row)}
          </tbody>
        </table>
      </div>
    );
  }
}
