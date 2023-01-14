@REM Script that sends requests to the server to generate PDFs

@REM Server = http://localhost:9000

@REM Example: http://localhost:9000/?day=10&month=10&year=2022
@REM This generates a PDF for the 10th of October 2022

@REM GENERATE PDFS FOR 2022

@REM Loop through the days
for /l %%a in (1,1,31) do (
  @REM Loop through the months
  for /l %%b in (1,1,12) do (
    @REM Loop through the years
    for /l %%c in (2022,1,2022) do (
      @REM Generate the PDF for the date
      @REM
      @REM Note: You may need to escape the & character with a ^
      @REM This is because the & character is a special character in the command prompt
      @REM   
      @REM Example: http://localhost:9000/?day=10^&month=10^&year=2022
      @REM
      curl http://localhost:9000/?day=%%a^&month=%%b^&year=%%c
    )
  )
)