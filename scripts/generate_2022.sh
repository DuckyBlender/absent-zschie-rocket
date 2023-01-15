#!/bin/sh

# Script that sends requests to the server to generate PDFs
# Server = http://localhost:5000
# Example: http://localhost:5000/?day=10&month=10&year=2022
# This generates a PDF for the 10th of October 2022

# GENERATE PDFS FOR 2022

# Loop through the days
for a in $(seq 1 31); do
  # Loop through the months
  for b in $(seq 1 12); do
    # Loop through the years
    for c in 2022; do
      # Generate the PDF for the date
      #
      # Note: You may need to escape the & character with a \
      # This is because the & character is a special character in the shell
      #   
      # Example: http://localhost:5000/?day=10\&month=10\&year=2022
      #
      curl http://localhost:5000/?day=$a\&month=$b\&year=$c
    done
  done
done