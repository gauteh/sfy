#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Tue Apr  4 17:29:59 2023

@author: susanne
"""

import matplotlib.pyplot as plt
import pandas as pd

#%% open CVS

df1 = pd.read_csv('/Users/susanne/Documents/MetOffice/CurrentRanger/test1.csv', sep=',')

df2 = pd.read_csv('/Users/susanne/Documents/MetOffice/CurrentRanger/test_drifter.csv', sep=',')

#%%

print(df1.columns.tolist())

timestamp1 = df1['Timestamp']
amps1 = df1[' Amps']

timestamp2 = df2['Timestamp']
amps2 = df2[' Amps']


#%%

df1['Date'] = pd.to_datetime(df1["Timestamp"]).dt.date
df1['Time'] = pd.to_datetime(df1['Timestamp']).dt.time


df2['Date'] = pd.to_datetime(df2["Timestamp"]).dt.date
df2['Time'] = pd.to_datetime(df2['Timestamp']).dt.time

#%%


#filtering only the minuts you want. Could change minute for hour or second
df1_new=df1[(df1["Time"].apply(lambda x : x.minute)>19) & (df1["Time"].apply(lambda x : x.minute)<23)]
df2_new=df2[(df2["Time"].apply(lambda x : x.minute)>30) & (df2["Time"].apply(lambda x : x.minute)<57)]


#%%


#timeFmt = mdates.DateFormatter('%H:%M:%S')

#%%

time1_new = df1_new['Time']
amps1_new = df1_new[' Amps']

time2_new = df2_new['Time']
amps2_new = df2_new[' Amps']


#%%

amps2_cut = amps2.loc[1000000:5000000]
amps1_cut = amps1.loc[1000000:2396186]
#%%


fig, ax = plt.subplots(figsize=(10,5))

amps1_cut.plot()
amps2_cut.plot()




