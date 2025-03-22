#!/bin/bash

# Reset all attributes
RESET="\e[0m"

# Text styles
BOLD="\e[1m"
DIM="\e[2m"
ITALIC="\e[3m"         # May not be supported in all terminals.
UNDERLINE="\e[4m"      # Standard single underline
REVERSE="\e[7m"
STRIKETHROUGH="\e[9m"  # Not supported in all terminal emulators.

# Extended underline styles (if supported)
# Using the extended SGR syntax for underline styles:
# Single: \e[4m (or \e[4:1m), Double: \e[4:2m, Curly: \e[4:3m, Dotted: \e[4:4m
DOUBLE_UNDERLINE="\e[4:2m"
CURLY_UNDERLINE="\e[4:3m"
DOTTED_UNDERLINE="\e[4:4m"

# Basic palette colors (foreground)
BLACK="\e[30m"
RED="\e[31m"
GREEN="\e[32m"
YELLOW="\e[33m"
BLUE="\e[34m"
MAGENTA="\e[35m"
CYAN="\e[36m"
WHITE="\e[37m"
BRIGHT_BLACK="\e[90m"
BRIGHT_RED="\e[91m"
BRIGHT_GREEN="\e[92m"
BRIGHT_YELLOW="\e[93m"
BRIGHT_BLUE="\e[94m"
BRIGHT_MAGENTA="\e[95m"
BRIGHT_CYAN="\e[96m"
BRIGHT_WHITE="\e[97m"

# Basic background colors
BG_BLACK="\e[40m"
BG_RED="\e[41m"
BG_GREEN="\e[42m"
BG_YELLOW="\e[43m"
BG_BLUE="\e[44m"
BG_MAGENTA="\e[45m"
BG_CYAN="\e[46m"
BG_WHITE="\e[47m"
BG_BRIGHT_BLACK="\e[100m"
BG_BRIGHT_RED="\e[101m"
BG_BRIGHT_GREEN="\e[102m"
BG_BRIGHT_YELLOW="\e[103m"
BG_BRIGHT_BLUE="\e[104m"
BG_BRIGHT_MAGENTA="\e[105m"
BG_BRIGHT_CYAN="\e[106m"
BG_BRIGHT_WHITE="\e[107m"

# List of text styles
printf "${BOLD}Available Text Styles:${RESET}\n"
printf "• Bold:          ${BOLD}This is bold text${RESET}\n"
printf "• Dim:           ${DIM}This is dim text${RESET}\n"
printf "• Italic:        ${ITALIC}This is italic text${RESET}\n"
printf "• Underline:     ${UNDERLINE}This is underlined text${RESET}\n"
printf "• Strikethrough: ${STRIKETHROUGH}This is strikethrough text${RESET}\n"
printf "• Reverse:       ${REVERSE}This is reversed text${RESET}\n"
printf "\n"

# Basic Foreground Colors
printf "${BOLD}Basic Foreground Colors:${RESET}\n"
printf "• Normal: "
printf "${BLACK}Black${RESET} "
printf "${RED}Red${RESET} "
printf "${GREEN}Green${RESET} "
printf "${YELLOW}Yellow${RESET} "
printf "${BLUE}Blue${RESET} "
printf "${MAGENTA}Magenta${RESET} "
printf "${CYAN}Cyan${RESET} "
printf "${WHITE}White${RESET}\n"
printf "• Bright: "
printf "${BRIGHT_BLACK}Black${RESET} "
printf "${BRIGHT_RED}Red${RESET} "
printf "${BRIGHT_GREEN}Green${RESET} "
printf "${BRIGHT_YELLOW}Yellow${RESET} "
printf "${BRIGHT_BLUE}Blue${RESET} "
printf "${BRIGHT_MAGENTA}Magenta${RESET} "
printf "${BRIGHT_CYAN}Cyan${RESET} "
printf "${BRIGHT_WHITE}White${RESET}\n\n"

# Mixing styles with colors
printf "${BOLD}Mixed Styles with Colors:${RESET}\n"
printf "• ${BOLD}${RED}Bold red text${RESET}\n"
printf "• ${ITALIC}${GREEN}Italic green text${RESET}\n"
printf "• ${UNDERLINE}${BLUE}Underlined blue text${RESET}\n"
printf "• ${BOLD}${UNDERLINE}${MAGENTA}Bold underlined magenta text${RESET}\n\n"

FG=${BLACK}
if [ "$1" == "light" ]; then
    FG=${BRIGHT_WHITE}
fi

# Basic Background Colors with contrasting text
printf "${BOLD}Basic Background Colors:${RESET}\n"
printf "${BG_BLACK}${WHITE} Black BG ${RESET}"
printf "${BG_RED}${FG} Red BG ${RESET}"
printf "${BG_GREEN}${FG} Green BG ${RESET}"
printf "${BG_YELLOW}${BLACK} Yellow BG ${RESET}"
printf "${BG_BLUE}${FG} Blue BG ${RESET}"
printf "${BG_MAGENTA}${FG} Magenta BG ${RESET}"
printf "${BG_CYAN}${FG} Cyan BG ${RESET}"
printf "${BG_WHITE}${BLACK} White BG ${RESET}\n\n"

# 24-bit True Color Background Gradient (79 characters wide)
printf "${BOLD}24-bit True Color Background:${RESET}\n"
# Loop 93 times to print exactly 79 blocks.
for i in $(seq 0 78); do
    # Calculate red value decreasing from 255 to 0 across the gradient
    red=$(printf "%.0f" "$(echo "255 - (255 * $i / 78)" | bc -l)")
    # Calculate blue value increasing from 0 to 255 across the gradient
    blue=$(printf "%.0f" "$(echo "(255 * $i / 78)" | bc -l)")
    # Print one block (a space with the background color set) without a newline.
    printf "\e[48;2;${red};0;${blue}m \e[0m"
done
printf "\n"
