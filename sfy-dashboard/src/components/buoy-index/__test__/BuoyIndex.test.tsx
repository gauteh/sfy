import {render} from 'inferno';
import {BuoyIndex} from '../BuoyIndex';

import * as api from 'hub';
jest.mock('hub');


it('renders without crashing', () => {
  const div = document.createElement("div");
  render(<BuoyIndex/>, div);
});

