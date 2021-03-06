import {Component, render, createRef, VNode} from 'inferno';
import {BuoyIndex} from './components/buoy-index/BuoyIndex';
import {Login} from './components/login';
import Cookies from 'js-cookie'
import cx from 'classnames';
import 'bootstrap/scss/bootstrap.scss';
import './main.css';
import * as hub from 'hub';

const container = document.getElementById('app');

interface State {
  token: string;
}

class Dashboard extends Component<any, any> {
  public state = {
    token: undefined
  };

  constructor(props, context) {
    super(props, context);
    this.state.token = Cookies.get('token');
    hub.API_CONF.setToken(this.state.token);
  }

  setToken = (token: string) => {
    Cookies.set('token', token, { expires: 30, path: '' });
    hub.API_CONF.setToken(token);
    this.setState({token: token});
  };

  clearToken = () => {
    Cookies.remove('token');
    this.setState({token: undefined});
    hub.API_CONF.setToken(undefined);
  };

  public render() {
    return (
      <div id="main-container" class="container-fluid mh-100 d-flex flex-column h-100" style="height: 100vh">
        <div class="flex-shrink-0">
          { (this.state.token === undefined) ? (<Login cbToken={ this.setToken }/>) : (<BuoyIndex />) }
        </div>

        <footer class={cx ('footer', 'mt-auto', 'py-1', 'bg-light', { 'd-none' : this.state.token === undefined })}>
          <div class="container-fluid d-flex flex-row px-2">
            <button type="button" class="btn btn-outline-dark btn-sm" onClick={this.clearToken} >✕ Log out</button>
            <button type="button" class="btn btn-link"><a href="https://github.com/gauteh/sfy">github.com/gauteh/sfy</a></button>
          </div>
        </footer>
      </div>
    );
  }
}

render(<Dashboard />, container);

