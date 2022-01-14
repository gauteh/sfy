import { Component, render, createRef, VNode } from 'inferno';
import { cloneVNode } from 'inferno-clone-vnode';
import * as mousetrap from 'mousetrap';
import { BuoyIndex } from './components/buoy-index/BuoyIndex';

import 'bootstrap/dist/css/bootstrap.css';
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
    <div class="container">
      <h1> SFY </h1>
      <BuoyIndex />
    </div>
    );
  }
}

render(<Dashboard />, container);

