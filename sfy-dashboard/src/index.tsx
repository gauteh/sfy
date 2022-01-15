import {Component, render, createRef, VNode} from 'inferno';
import {BuoyIndex} from './components/buoy-index/BuoyIndex';

import 'bootstrap/scss/bootstrap.scss';
import './main.css';

const container = document.getElementById('app');

interface State {
  active: number;
  buffers: VNode[];
}

class Dashboard extends Component<any, any> {
  public state = {
    loggedin: 0,
    token: ""
  };

  constructor(props, context) {
    super(props, context);
  }

  public render() {
    return (
      <div id="main-container" class="container-fluid">
        <BuoyIndex />
      </div>
    );
  }
}

render(<Dashboard />, container);

