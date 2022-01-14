import {Component, createRef, VNode} from 'inferno';

import moment from 'moment';
import cx from 'classnames';

import {finalize, tap} from 'rxjs/operators';
import {Buoy} from 'models';
import * as hub from 'hub';

import {findDOMNode} from 'inferno-extras';
import {renderToString} from 'inferno-server';
import ClusterizeJS from 'clusterize.js';

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

  public loadBuoys = () => {
    console.log('loading..');
    this.state.buoys.length = 0;
    this.setState({buoys: []});
  }

  public Row(props) {
    const buoy = props.buoy;

    const formatDate = (date: number): JSX.Element => {
      return (<span>{moment(new Date(date * 1000)).fromNow()}</span>);
    };

    return (
      <tr id={"t" + buoy.dev}
        key={buoy.dev}>
        <td class="ti-dev">
          {buoy.dev}
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
        <h2>Index</h2>
        <table class="ti table table-dark table-borderless table-sm">
          <tbody>
          </tbody>
        </table>
      </div>
    );
  }
}
