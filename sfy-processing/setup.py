#!/usr/bin/env python

import setuptools

setuptools.setup(
    name='sfy-processing',
    version='0.1.0',
    description='Processing tools for SFY',
    author='Gaute Hope',
    author_email='gauteh@met.no',
    url='http://github.com/gauteh/sfy',
    packages=setuptools.find_packages(),
    include_package_data=False,
    setup_requires=['setuptools_scm'],
    entry_points={
        'console_scripts': [
            'sfydata=sfy.cli.sfydata:sfy'
            ]
        }

)
