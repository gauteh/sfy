import { Component, } from 'react';
import Cookies from 'js-cookie'
import cx from 'classnames';
import * as hub from './hub';

import { BuoyIndex } from './components/buoy-index/BuoyIndex';
import { Login } from './components/login';

interface State {
  token: string | undefined;
}

export class Dashboard extends Component<{}, State> {
  public state: State = {
    token: undefined
  };

  constructor(props: {}) {
    super(props);
    this.state.token = Cookies.get('token');
    hub.API_CONF.setToken(this.state.token);
  }

  setToken = (token: string) => {
    Cookies.set('token', token, { expires: 30, path: '' });
    hub.API_CONF.setToken(token);
    this.setState({ token: token });
  };

  clearToken = () => {
    Cookies.remove('token');
    this.setState({ token: undefined });
    hub.API_CONF.setToken(undefined);
  };

  public render() {
    return (
    <div id="main-container" className="container-fluid mh-100 d-flex flex-column h-100" style={{'height': '100vh'}}>
        <div className="flex-shrink-0">
          {(this.state.token === undefined) ? (<Login cbToken={this.setToken} />) : (<BuoyIndex />)}
        </div>

        <footer className={cx('footer', 'mt-auto', 'py-1', 'bg-light', { 'd-none': this.state.token === undefined })}>
          <div className="container-fluid d-flex flex-row px-2">
            <button type="button" className="btn btn-outline-dark btn-sm" onClick={this.clearToken} >✕ Log out</button>
            <button type="button" className="btn btn-link"><a href="https://github.com/gauteh/sfy">github.com/gauteh/sfy</a></button>
          </div>
        </footer>
      </div>
    );
  }
}

