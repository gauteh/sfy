import { Component, } from 'react';
import Cookies from 'js-cookie'
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
    if (this.state.token !== undefined) {
      return <BuoyIndex onLogout={this.clearToken} />;
    }

    return (
      <div className="d-flex flex-column align-items-center justify-content-center" style={{ minHeight: '100dvh' }}>
        <Login cbToken={this.setToken} />
        <footer className="mt-auto py-2 text-center">
          <a href="https://github.com/gauteh/sfy" className="text-muted small">github.com/gauteh/sfy</a>
        </footer>
      </div>
    );
  }
}

