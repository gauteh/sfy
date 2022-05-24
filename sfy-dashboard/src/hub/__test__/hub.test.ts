import * as hub from '../';
import {TEST_CONF} from '../test.conf';

describe ('buoys api', () => {
  test ('fetch list of buoys', async () => {
    (global as any).fetch = require ('node-fetch');
    const b = await hub.get_buoys(TEST_CONF).toPromise();
    expect (b.length).toBeGreaterThanOrEqual (1);
  });
});


describe ('buoys api', () => {
  test ('fetch first buoy', async () => {
    (global as any).fetch = require ('node-fetch');
    const bs = await hub.get_buoys(TEST_CONF).toPromise();

    const bi = await hub.get_buoy(TEST_CONF, bs[0], b[1]).toPromise();
    expect(bi.files.length > 1);
    // console.log("files: " + bi.files.length);
  });
});


describe ('buoys api', () => {
  test ('fetch file', async () => {
    (global as any).fetch = require ('node-fetch');
    const bs = await hub.get_buoys(TEST_CONF).toPromise();

    const bi = await hub.get_buoy(TEST_CONF, bs[0], b[1]).toPromise();
    expect(bi.files.length > 1);

    const fi = await hub.get_file(TEST_CONF, bi.dev, bi.files[0]).toPromise();
    expect(fi.sn == 'cain');
  });
});
