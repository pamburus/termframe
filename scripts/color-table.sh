#!/bin/bash
#
#   This file echoes a bunch of color codes to the
#   terminal to demonstrate what's available.  Each
#   line is the color code of one foreground color,
#   out of 17 (default + 16 escapes), followed by a
#   test use of that color on all nine background
#   colors (default + 8 escapes).
#
#   Copied from http://tldp.org/HOWTO/Bash-Prompt-HOWTO/x329.html
T='gYw'   # The test text

printf "\n         def     40m     41m     42m     43m     44m     45m     46m     47m\n";

for FGs in '    m' '   1m' '  30m' '1;30m' '  31m' '1;31m' '  32m' \
           '1;32m' '  33m' '1;33m' '  34m' '1;34m' '  35m' '1;35m' \
           '  36m' '1;36m' '  37m' '1;37m';

  do FG=${FGs// /}
  printf " $FGs \033[$FG  $T  "

  for BG in 40m 41m 42m 43m 44m 45m 46m 47m;
    do printf "$EINS \033[$FG\033[$BG  $T  \033[0m";
  done
  echo;
done

printf "\n         def    100m    101m    102m    103m    104m    105m    106m    107m\n";

for FGs in '    m' '   1m' '  30m' '1;30m' '  31m' '1;31m' '  32m' \
           '1;32m' '  33m' '1;33m' '  34m' '1;34m' '  35m' '1;35m' \
           '  36m' '1;36m' '  37m' '1;37m';

  do FG=${FGs// /}
  printf " $FGs \033[$FG  $T  "

  for BG in 100m 101m 102m 103m 104m 105m 106m 107m;
    do printf "$EINS \033[$FG\033[$BG  $T  \033[0m";
  done
  echo;
done
