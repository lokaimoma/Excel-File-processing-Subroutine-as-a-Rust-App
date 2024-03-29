# Excel File processing Subroutine as a Rust App
I had a task of 
+ Sorting rows in an excel file, such that the sorting in preceeding column is not disrupted by sorting in subsequent columns
+ Finding words/letters in all cells and highlighting them
+ Finding words (which I call contractions) specified in a similar excel file like the one I'm processing (but without any actual data but the same column headers with rows specifying one or more values) to highligth their background

**Sample data**
![Sample data](https://github.com/lokaimoma/Rust-backend-for-processing-excel-file/assets/46226886/528c0431-7392-46c0-9960-545f4617ed1a)
**Contraction file**
![A contraction file](https://github.com/lokaimoma/Rust-backend-for-processing-excel-file/assets/46226886/9da1c80b-2ed5-43ed-b340-7e72875f4e3d)
**Result after the operation**
![The resulting file after the operation](https://github.com/lokaimoma/Rust-backend-for-processing-excel-file/assets/46226886/c7f15673-a730-4bfc-a849-4e7609bd234a)



## How to run

- Install the Rust language for your operating system. Step by step instructions can be found [here](https://www.rust-lang.org/tools/install)
- Unzip the project and change into project directory
- Run the command `cargo run --release` To run the project in release build mode

## Frontend

- Open the URL `http://127.0.0.1:6070` in the browser to use the frontend interface

## Routes

- There are three routes in total
- `/upload` For uploading the excel file
  - Post request
  - It expects a multipart form that contains the excel file
  - The method returns a JSON response of the form `{"id": "018d3fc6-10b0-7a01-9b84-6c7195fd052f"}`
  - The id can be used in the subsequent operations to avoid sending the file multiple times
- `/getHeader/file_id` To get the header column of the excel file
  - Get request
  - This route is needed to show the header row in the frontend after `start job` is clicked, you wouldn't need it if you're not using the frontend.
  - `file_id` has to be replaced with the `id` you got from `/upload` response
- `/runJob` To run the final job, returns the final contraction file as a downloadable attachement.
  - Post request
  - It expects a multipart form as the request body with the following parts.
    - `fileId` The file id from `/upload` response
    - `contractionFile` The contraction file for highlighting **This field is optional**
    - `sortCol` The columns to sort, it expects a value of structure `order,column_number`. The `order` can be either **asc** for ascending order sorting and **desc** for descending order sorting. Example: `asc,1` To sort the column 1 by ascending order. **When passing the column number, counting starts from 1 not 0**. You can append **multiple** `sortCol` values to your form.
    - `searchTerm` The text to search and highlight in the excel file. You can append **multiple** `searchTerm` values to your form.
    - `checkDate` Column number of columns to validate their date. This is just the column number nothing more. Example `1` for column 1. **When passing the column number, counting starts from 1 not 0**. You can append **multiple** `checkDate` values to your form.

* `/swagger-ui` To access the swagger ui

## URL
**The app runs on http://127.0.0.1:6070 by default**
