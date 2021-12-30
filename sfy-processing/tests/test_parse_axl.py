from sfy import axl

def test_parse_table():
    d = open('tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json').read()
    a = axl.Axl.parse(d)
    print(a)

    assert len(a.x) == 1024
    assert len(a.y) == 1024
    assert len(a.z) == 1024


