#!/bin/bash
git clone -o stable git://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git
cd linux
git remote add -f mainline git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
git remote add -f next git://git.kernel.org/pub/scm/linux/kernel/git/next/linux-next.git
git remote add -f net-next git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net-next.git
git remote add -f bluetooth-next git://git.kernel.org/pub/scm/linux/kernel/git/bluetooth/bluetooth-next.git
