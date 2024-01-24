"use strict";

const SERVER_URL = "http://localhost:6070";
const excelFileField = document.querySelector("#excelFile");
const excelFileName = document.querySelector("#fileName");
const contractionFile = document.querySelector("#contraFile");
const uploadStat = document.querySelector("#uploadStat");
const startJob = document.querySelector("#startJob");
const colHeaderCont = document.querySelector("#headerDisp");

startJob.addEventListener("click", function (_) {
  startJob.disabled = true;
  startJob.textContent = "Please wait";
  const excelFileForm = document.querySelector("#excelFileForm");
  const form = new FormData(excelFileForm);
  fetch(`${SERVER_URL}/upload`, {
    method: "post",
    body: form,
  })
    .then((response) => {
      console.log(response);
      if (!response.ok) {
        response.text().then((txt) => {
          console.error(txt);
        });
      } else {
        console.log("File uploaded successfully");
        return response.json();
      }
    })
    .then((json) => {
      const excelFileField = json["id"];
      getHeaderRow(excelFileField).then((json) => {
        const documentFragment = document.createDocumentFragment();
        for (const columnTitle of json["columns"]) {
          const p = document.createElement("p");
          p.textContent = columnTitle;
          documentFragment.appendChild(p);
        }
        colHeaderCont.appendChild(documentFragment);
      });
    })
    .finally(() => {
      startJob.disabled = false;
      startJob.textContent = "Start Job";
    });
});

/**
 *
 * @param {String} id
 * @returns {Promise<String[]>}
 */
function getHeaderRow(id) {
  return fetch(`${SERVER_URL}/getHeader/${id}`).then((response) => {
    if (!response.ok) {
      response.text().then((txt) => console.error(txt));
    } else {
      console.log("Header request successful");
      return response.json();
    }
  });
}

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
     * @type {HtmlElement | Undefined}
     */
    const oppelement = document.querySelector(`#${opp_element}`);

    if (oppelement) {
      oppelement.checked = false;
    }
  }
}
