# How To

*This file may go stale as commands change/appear/die, running \[help \<command\>\] is always the best way to get fresh help on how to use a command*


We assume the prefix is "!scriv" for the duration of this file

## Get Help, view the commands
```
!scriv help
```
Get a more descriptive help for a command by putting the command name after, like `!scriv help show-stats`


## First Up


The bot looks back through the history of channels and then keeps up to date with messages once they are "initialised"
To add a channel run:
```
!scriv init-channel #channel-name
```
e.g. `!scriv init-channel #the-fall-of-rome`

If a channel is big, it may take some time fetching and processing all the old messages - please be patient!
This command is limited by a role `MasterScrivener`, because initialising channels can put some load on the bot - Be sure you ask your friendly admin if you want to add a channel!


##Get me stats!

You can now get stats for any initialised channels
```
!scriv show-stats #channel-name
```
e.g. `!scriv show-stats #the-fall-of-rome`

---

You can generate a wordcloud for all or a specific user in a channel

```
!scriv gen-wordcloud #channel-name @User mask
```
The masks are some pre-defined list of shapes, see what you prefer!

e.g. `!scriv gen-wordcloud #the-fall-of-room @Caligula horse` 

---

 View a users stats across all channels on the server
 ```
!scriv server-summary @User
```
e.g. `!scriv server-summary @Caligula`