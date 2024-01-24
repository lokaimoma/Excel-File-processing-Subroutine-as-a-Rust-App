"use strict";

const SERVER_URL = "http://localhost:6070";
const excelFileField = document.querySelector("#excelFile");
const excelFileName = document.querySelector("#fileName");
const contractionFile = document.querySelector("#contraFile");
const uploadStat = document.querySelector("#uploadStat");
const startJob = document.querySelector("#startJob");

startJob.addEventListener("click", function (_) {
  startJob.disabled = true;
  startJob.textContent = "Please wait";
  const excelFileForm = document.querySelector("#excelFileForm");
  const form = new FormData(excelFileForm);
  fetch(`${SERVER_URL}/upload`, {
    method: "post",
    body: form,
    mode: "no-cors",
  })
    .then((response) => {
      console.log(response);
      if (!response.ok) {
        response.text().then((txt) => {
          console.error(txt);
        });
      } else {
        console.log("File uploaded successfully");
      }
      return response.json();
    })
    .then((json) => {
      console.log(json);
    })
    .finally(() => {
      startJob.disabled = false;
      startJob.textContent = "Start Job";
    });
});

excelFileField.addEventListener("change", function (e) {
  const target = e.target;
  if (target.files.length == 0) {
    excelFileName.textContent = "No file selected";
    startJob.disabled = true;
  } else {
    excelFileName.textContent = target.files[0].name;
    startJob.disabled = false;
  }
});

contractionFile.addEventListener("change", function (e) {
  const target = e.target;
  if (target.files.length == 0) {
    uploadStat.classList.remove("uploaded");
  } else {
    uploadStat.classList.add("uploaded");
  }
});

/**
 *
 * @param {Event} event
 */
function uncheck_counter(event) {
  /**
   * @param {HTMLElement}
   */
  const target = event.target;
  if (!target.checked) {
    return;
  }

  /**
   * @type(string | undefined)
   */
  const opp_element = target.dataset.opp;
  if (opp_element) {
    /**
     * @type(HtmlElement | undefined)
     */
    const oppelement = document.querySelector(`#${opp_element}`);

    if (oppelement) {
      oppelement.checked = false;
    }
  }
}
