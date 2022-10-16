import { Component } from 'react';

interface State {
  token: string;
}

interface Props {
  cbToken: (token: string) => void,
}

export class Login extends Component<Props, State> {
  constructor(props: any) {
    super(props);
  }

  on_keyup = (event: any) => {
    if (event.keyCode === 13) {
      this.on_click(event);
    }
  };

  on_click = (_event: any) => {
    let input = document.getElementById('token-input') as HTMLInputElement;
    let token = input.value;
    input.value = "";

    this.props.cbToken(token);
  };

  public render() {
    return (
      <div className="container d-flex justify-content-center">
        <div className="card bg-dark text-light" style={{'width': '18rem'}}>
          <div className="card-body">
            <h5 className="card-title">Input token</h5>
            <div className="input-group mb-3">
              <span className="input-group-text" id="basic-addon1">🔑</span>
              <input autoFocus id="token-input" type="text" className="form-control" placeholder="Token" aria-label="Token" aria-describedby="basic-addon1" onKeyUp={this.on_keyup} />
            </div>
            <a href="#" className="btn btn-primary" onClick={this.on_click}>Go</a>
          </div>
        </div>
      </div>
    );
  }
}



