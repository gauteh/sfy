from sfy import event


def test_parse_track():
    d = open(
        'tests/data/1642676373013-b52920fb-eef5-4796-8e38-198a56741379__track.qo.json'

    ).read()
    a = event.Event.parse(d)
    print(a)

def test_parse_log():
    d = open(
        'tests/data/1642676372928-a9b6422f-cea5-4e4c-8b56-248495756115__log.qo.json'

    ).read()
    a = event.Event.parse(d)
    print(a)

