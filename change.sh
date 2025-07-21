#!/bin/sh

git filter-branch --env-filter '
OLD_EMAIL="kurylko.m@ajax.systems"
CORRECT_NAME="CowEstaris"
CORRECT_EMAIL="vk_vm@ukr.net"
if [ "$GIT_COMMITTER_EMAIL" = "$OLD_EMAIL" ]
then
    export GIT_COMMITTER_NAME="$CORRECT_NAME"
    export GIT_COMMITTER_EMAIL="$CORRECT_EMAIL"
fi
if [ "$GIT_AUTHOR_EMAIL" = "$OLD_EMAIL" ]
then
    export GIT_AUTHOR_NAME="$CORRECT_NAME"
    export GIT_AUTHOR_EMAIL="$CORRECT_EMAIL"
fi
' --tag-name-filter cat -- --branches --tags
'[Kawaiika-Raws] Shingeki no Kyojin (2013) 08 [BDRip 1920x1080 HEVC FLAC]'
'[Kawaiika-Raws] Shingeki no Kyojin (2013) 20 [BDRip 1920x1080 HEVC FLAC]'