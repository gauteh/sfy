import {Component, render, createRef, VNode} from 'inferno';

interface State {
  token: string;
}

interface Props {
  cbToken: (token: string) => void,
}

export class Login extends Component<Props, any> {
  constructor(props, context) {
    super(props, context);
  }

  on_keyup = (event) => {
    if (event.keyCode === 13) {
      this.on_click(event);
    }
  };

  on_click = (_event) => {
    let input = document.getElementById('token-input') as HTMLInputElement;
    let token = input.value;
    input.value = "";

    this.props.cbToken(token);
  };

  public render() {
    return (
      <div class="container d-flex justify-content-center">
        <div class="card bg-dark text-light" style="width: 18rem;">
          <div class="card-body">
            <h5 class="card-title">Input token</h5>
            <div class="input-group mb-3">
              <span class="input-group-text" id="basic-addon1">🔑</span>
              <input autoFocus id="token-input" type="text" class="form-control" placeholder="Token" aria-label="Token" aria-describedby="basic-addon1" onkeyup={this.on_keyup}/>
            </div>
            <a href="#" class="btn btn-primary" onclick={ this.on_click }>Go</a>
          </div>
        </div>
      </div>
    );
  }
}


