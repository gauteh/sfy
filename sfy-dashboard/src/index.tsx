import {Component, render, createRef, VNode} from 'inferno';
import {BuoyIndex} from './components/buoy-index/BuoyIndex';
import {Login} from './components/login';

import 'bootstrap/scss/bootstrap.scss';
import './main.css';

const container = document.getElementById('app');

interface State {
  token: string;
}

class Dashboard extends Component<any, any> {
  public state = {
    token: null
  };

  constructor(props, context) {
    super(props, context);
  }

  setToken = (token: string) => {
    this.setState({token: token});
  };

  public render() {
    return (
      <div id="main-container" class="container-fluid">
        { (this.state.token == null) ? (<Login cbToken={ this.setToken }/>) : (<BuoyIndex />) }
      </div>
    );
  }
}

render(<Dashboard />, container);

